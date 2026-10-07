//! Fail closed on any workload other than one completed 128/128 request.
use serde_json::Value;

#[derive(Default)]
pub(super) struct RequestGuard {
    request: Option<u64>,
    admitted: bool,
    completed: bool,
}

impl RequestGuard {
    pub(super) fn observe(&mut self, value: &Value) -> Result<(), String> {
        let reject = || "decode diagnostic requires exactly one completed uncached 128/128 request".to_owned();
        match value["event"].as_str() {
            Some("queued") => {
                let request = value["request_id"].as_u64().filter(|id| *id != 0).ok_or_else(reject)?;
                if self.request.is_some() || value["prompt_tokens"] != 128 { return Err(reject()); }
                self.request = Some(request);
            }
            Some("admission") => {
                if self.request.is_none() || self.request != value["request_id"].as_u64()
                    || self.admitted || value["prompt_token_count"] != 128 || value["cached_tokens"] != 0 {
                    return Err(reject());
                }
                self.admitted = true;
            }
            Some("request") => {
                if !self.admitted || self.completed || self.request != value["request_id"].as_u64()
                    || value["state"] != "Completed" || value["prompt_token_count"] != 128
                    || value["generated_tokens"].as_array().is_none_or(|tokens| tokens.len() != 128)
                    || value["cached_prefix_tokens"] != 0 {
                    return Err(reject());
                }
                self.completed = true;
            }
            Some("rejected" | "cancel") => return Err(reject()),
            _ => {}
        }
        Ok(())
    }

    pub(super) fn finish(&self) -> Result<(), String> {
        if self.request.is_none() || !self.admitted || !self.completed {
            return Err("decode diagnostic request incomplete".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn events() -> [Value;3] {
        [json!({"event":"queued","request_id":1,"prompt_tokens":128}),
         json!({"event":"admission","request_id":1,"prompt_token_count":128,"cached_tokens":0}),
         json!({"event":"request","request_id":1,"state":"Completed","prompt_token_count":128,
            "generated_tokens":vec![1;128],"cached_prefix_tokens":0})]
    }

    #[test]
    fn one_exact_request_only() {
        let mut guard = RequestGuard::default();
        for value in events() { guard.observe(&value).unwrap(); }
        guard.finish().unwrap();
        assert!(guard.observe(&events()[0]).is_err());
        assert!(guard.observe(&events()[2]).is_err());
    }

    #[test]
    fn empty_and_partial_requests_fail() {
        let mut guard = RequestGuard::default();
        assert!(guard.finish().is_err());
        for value in &events()[..2] { guard.observe(value).unwrap(); }
        assert!(guard.finish().is_err());
        let mut short = events()[2].clone();
        short["generated_tokens"] = json!(vec![1;127]);
        assert!(guard.observe(&short).is_err());
    }

    #[test]
    fn mismatched_identity_prefix_and_prompt_fail() {
        for (index,key,replacement) in [(0,"prompt_tokens",json!(127)),(1,"request_id",json!(2)),
            (1,"cached_tokens",json!(32)),(2,"state",json!("Cancelled")),(2,"cached_prefix_tokens",json!(32))] {
            let mut guard = RequestGuard::default();
            let mut values = events();
            for value in &values[..index] { guard.observe(value).unwrap(); }
            values[index][key] = replacement;
            assert!(guard.observe(&values[index]).is_err());
        }
    }
}
