//! Explicit local estimates; these are not cache counters reported by Kiro.
use std::sync::Arc;
use serde_json::{Value, json};
use super::cache_tracker::{CacheProfile, CacheResult, CacheTracker};
use crate::kiro::provider::UsedCredential;

pub struct PendingCache {
    tracker: Arc<CacheTracker>,
    profile: CacheProfile,
}

pub struct CacheAccounting {
    pending: PendingCache,
    credential_id: u64,
    result: CacheResult,
    failed: bool,
    saw_response: bool,
    committed: bool,
}

impl PendingCache {
    pub fn new(tracker: Arc<CacheTracker>, payload: &super::types::MessagesRequest, total: i32) -> Self {
        let profile = tracker.build_profile(payload, total);
        Self { tracker, profile }
    }

    pub fn resolve(self, response: &reqwest::Response) -> Option<CacheAccounting> {
        // Missing identity must never fall back to a shared credential (e.g. ID 0).
        let credential_id = response.extensions().get::<UsedCredential>()?.0;
        let result = self.tracker.compute(credential_id, &self.profile);
        Some(CacheAccounting { pending: self, credential_id, result, failed: false, saw_response: false, committed: false })
    }
}

impl CacheAccounting {
    pub fn observe(&mut self, event: &crate::kiro::model::events::Event) {
        use crate::kiro::model::events::Event;
        match event {
            Event::AssistantResponse(_) | Event::ToolUse(_) => self.saw_response = true,
            Event::Error { .. } | Event::Exception { .. } => self.failed = true,
            _ => {}
        }
    }
    pub fn fail(&mut self) { self.failed = true; }
    pub fn commit(&mut self) {
        if !self.failed && self.saw_response && !self.committed {
            self.pending.tracker.update(self.credential_id, &self.pending.profile);
            self.committed = true;
        }
    }
    pub fn apply(&self, usage: &mut Value, total: i32) {
        apply_usage(usage, total, self.result);
    }
}

fn apply_usage(usage: &mut Value, total: i32, result: CacheResult) {
    let total = total.max(0);
    let read = result.cache_read_input_tokens.clamp(0, total);
    let write = result.cache_creation_input_tokens.clamp(0, total - read);
    let five_minute = result.cache_creation_5m_input_tokens.clamp(0, write);
    let one_hour = result.cache_creation_1h_input_tokens.clamp(0, write - five_minute);
    usage["input_tokens"] = json!(total - read - write);
    usage["cache_read_input_tokens"] = json!(read);
    usage["cache_creation_input_tokens"] = json!(write);
    usage["cache_creation"] = json!({"ephemeral_5m_input_tokens": write - one_hour, "ephemeral_1h_input_tokens": one_hour});
    usage["cache_accounting"] = json!({"source":"local_estimate", "upstream_reported":false});
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_split_conserves_total_and_preserves_output() {
        let mut usage = json!({"input_tokens":100,"output_tokens":7});
        apply_usage(&mut usage, 100, CacheResult {cache_read_input_tokens:60,cache_creation_input_tokens:30,cache_creation_5m_input_tokens:20,cache_creation_1h_input_tokens:10});
        assert_eq!(usage["input_tokens"],10);
        assert_eq!(usage["cache_read_input_tokens"],60);
        assert_eq!(usage["cache_creation_input_tokens"],30);
        assert_eq!(usage["output_tokens"],7);
        assert_eq!(usage["cache_creation"]["ephemeral_1h_input_tokens"],10);
        assert_eq!(usage["cache_accounting"]["source"],"local_estimate");
    }
    #[test]
    fn cache_split_clamps_estimates_to_reported_total() {
        let mut usage=json!({});
        apply_usage(&mut usage,10,CacheResult {cache_read_input_tokens:20,cache_creation_input_tokens:30,..Default::default()});
        assert_eq!(usage["input_tokens"],0);
        assert_eq!(usage["cache_read_input_tokens"],10);
        assert_eq!(usage["cache_creation_input_tokens"],0);
    }
    #[test]
    fn unsuccessful_or_incomplete_requests_do_not_warm_cache() {
        let tracker=Arc::new(CacheTracker::new(std::time::Duration::from_secs(300)));
        let payload: super::super::types::MessagesRequest=serde_json::from_value(json!({"model":"claude-sonnet-4-5-20250929","max_tokens":8,"system":[{"type":"text","text":"cache ".repeat(3000),"cache_control":{"type":"ephemeral"}}],"messages":[{"role":"user","content":"OK"}]})).unwrap();
        let pending=PendingCache::new(tracker.clone(),&payload,10000);
        let result=tracker.compute(1,&pending.profile);
        let mut accounting=CacheAccounting {pending,credential_id:1,result,failed:false,saw_response:false,committed:false};
        accounting.commit();
        assert_eq!(tracker.compute(1,&accounting.pending.profile).cache_read_input_tokens,0);
        accounting.saw_response=true; accounting.fail(); accounting.commit();
        assert_eq!(tracker.compute(1,&accounting.pending.profile).cache_read_input_tokens,0);
        accounting.failed=false; accounting.commit();
        assert!(tracker.compute(1,&accounting.pending.profile).cache_read_input_tokens>0);
        assert_eq!(tracker.compute(2,&accounting.pending.profile).cache_read_input_tokens,0);
    }
    fn fixture(tracker: Arc<CacheTracker>, credential: u64) -> CacheAccounting {
        let payload: super::super::types::MessagesRequest=serde_json::from_value(json!({"model":"claude-sonnet-4-5-20250929","max_tokens":8,"system":[{"type":"text","text":"cache ".repeat(3000),"cache_control":{"type":"ephemeral"}}],"messages":[{"role":"user","content":"OK"}]})).unwrap();
        let mut response = reqwest::Response::from(http::Response::new(""));
        response.extensions_mut().insert(UsedCredential(credential));
        PendingCache::new(tracker, &payload, 10000).resolve(&response).unwrap()
    }
    fn assistant_event() -> crate::kiro::model::events::Event {
        crate::kiro::model::events::Event::AssistantResponse(serde_json::from_value(json!({"content":"OK"})).unwrap())
    }
    #[test]
    fn regular_and_buffered_streams_report_hits_without_double_billing() {
        use super::super::stream::{StreamContext, BufferedStreamContext};
        let tracker=Arc::new(CacheTracker::new(std::time::Duration::from_secs(300)));
        let mut first=StreamContext::new_with_thinking("claude-sonnet-4-5-20250929",10000,false,Default::default());
        first.cache_accounting=Some(fixture(tracker.clone(),1));
        let start=first.create_message_start_event();
        assert!(start["message"]["usage"]["cache_creation_input_tokens"].as_i64().unwrap()>0);
        first.generate_initial_events(); first.process_kiro_event(&assistant_event());
        let final_events=first.generate_final_events();
        let usage=&final_events.iter().find(|event| event.event=="message_delta").unwrap().data["usage"];
        assert_eq!(usage["input_tokens"].as_i64().unwrap()+usage["cache_creation_input_tokens"].as_i64().unwrap()+usage["cache_read_input_tokens"].as_i64().unwrap(),10000);
        let mut second=BufferedStreamContext::new("claude-sonnet-4-5-20250929",10000,false,Default::default());
        second.set_cache_accounting(Some(fixture(tracker.clone(),1)));
        second.process_and_buffer(&assistant_event());
        let events=second.finish_and_get_all_events();
        for event in &events {
            let usage=if event.event=="message_start" {Some(&event.data["message"]["usage"])} else if event.event=="message_delta" {Some(&event.data["usage"])} else {None};
            if let Some(usage)=usage {
                assert!(usage["cache_read_input_tokens"].as_i64().unwrap()>0);
                assert_eq!(usage["cache_creation_input_tokens"],0);
                assert_eq!(usage["input_tokens"].as_i64().unwrap()+usage["cache_read_input_tokens"].as_i64().unwrap(),10000);
            }
        }
    }
    #[test]
    fn interrupted_streams_and_missing_identity_never_warm() {
        use super::super::stream::StreamContext;
        let tracker=Arc::new(CacheTracker::new(std::time::Duration::from_secs(300)));
        let mut stream=StreamContext::new_with_thinking("claude-sonnet-4-5-20250929",10000,false,Default::default());
        stream.cache_accounting=Some(fixture(tracker.clone(),1));
        stream.generate_initial_events(); stream.process_kiro_event(&assistant_event());
        stream.fail_cache(); stream.generate_final_events();
        assert_eq!(fixture(tracker.clone(),1).result.cache_read_input_tokens,0);
        let pending=fixture(tracker,1).pending;
        let response=reqwest::Response::from(http::Response::new(""));
        assert!(pending.resolve(&response).is_none());
    }

}
