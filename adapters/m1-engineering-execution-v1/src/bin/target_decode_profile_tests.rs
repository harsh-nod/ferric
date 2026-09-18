use super::*;

fn document() -> serde_json::Value {
    let mut tokens = vec![12095, 13];
    tokens.extend(0..30);
    let pass = serde_json::json!({
        "token_ids": tokens, "argmax_token_ids": tokens,
        "decoded_new_tokens": " Paris. independently fixed test suffix"
    });
    serde_json::json!({
        "format": "FERRIC-QWEN3-8B-GREEDY-REFERENCE-V1",
        "authority": "independent-offline-reference-only",
        "model": {
            "repository": "Qwen/Qwen3-8B", "revision": TARGET_REVISION,
            "deployment_bundle_identity": "fixture-bundle", "class": "Qwen3ForCausalLM",
            "config": { "hidden_size": 4096, "model_type": "qwen3",
                "num_attention_heads": 32, "num_hidden_layers": 36, "num_key_value_heads": 8,
                "torch_dtype": "torch.bfloat16", "vocab_size": 151936 }
        },
        "prompt": { "add_special_tokens": false, "text": PROMPT, "token_ids": PROMPT_TOKENS },
        "execution": {
            "attention_implementation": "sdpa", "device_dtype": "bfloat16", "do_sample": false,
            "max_new_tokens": 32, "model_forward": "transformers.generate", "network": "offline",
            "num_beams": 1, "use_cache": true,
            "output_loading_info": { "error_msgs": [], "mismatched_keys": [], "missing_keys": [], "unexpected_keys": [] },
            "passes": [pass, pass]
        }
    })
}

fn parse(value: serde_json::Value, new_tokens: u32) -> Result<Reference, String> {
    Reference::from_document(
        serde_json::from_value(value).map_err(|e| e.to_string())?,
        new_tokens,
    )
}

#[test]
fn reference_preserves_only_the_two_independently_frozen_byte_extents() {
    let prefix = parse(document(), 2).unwrap();
    assert_eq!(prefix.tokens, PREFIX_TOKENS);
    assert_eq!(prefix.bytes, PREFIX_BYTES);
    let full = parse(document(), 32).unwrap();
    assert_eq!(full.tokens.len(), 32);
    assert_eq!(full.bytes, b" Paris. independently fixed test suffix");
    for count in [0, 1, 3, 4, 16, 31, 33, u32::MAX] {
        assert!(parse(document(), count).is_err());
    }
}

#[test]
fn source_role_precision_revision_and_generation_are_closed() {
    for (pointer, replacement) in [
        ("/format", serde_json::json!("other")),
        ("/authority", serde_json::json!("candidate")),
        ("/model/repository", serde_json::json!("Qwen/Qwen3-0.6B")),
        ("/model/revision", serde_json::json!("other")),
        ("/model/class", serde_json::json!("OtherModel")),
        (
            "/model/config/torch_dtype",
            serde_json::json!("torch.float8_e4m3fn"),
        ),
        ("/model/config/num_hidden_layers", serde_json::json!(28)),
        ("/execution/device_dtype", serde_json::json!("float16")),
        (
            "/execution/attention_implementation",
            serde_json::json!("eager"),
        ),
        ("/execution/model_forward", serde_json::json!("candidate")),
        ("/execution/network", serde_json::json!("online")),
        ("/execution/do_sample", serde_json::json!(true)),
        ("/execution/num_beams", serde_json::json!(2)),
        ("/execution/use_cache", serde_json::json!(false)),
        ("/execution/max_new_tokens", serde_json::json!(2)),
        ("/prompt/add_special_tokens", serde_json::json!(true)),
        ("/prompt/text", serde_json::json!("Other prompt")),
        ("/prompt/token_ids/0", serde_json::json!(0)),
    ] {
        let mut value = document();
        *value.pointer_mut(pointer).unwrap() = replacement;
        assert!(parse(value, 2).is_err(), "accepted {pointer}");
    }
}

#[test]
fn all_source_loading_anomalies_are_rejected() {
    for key in [
        "error_msgs",
        "mismatched_keys",
        "missing_keys",
        "unexpected_keys",
    ] {
        let mut value = document();
        value["execution"]["output_loading_info"][key] = serde_json::json!(["error"]);
        assert!(parse(value, 32).is_err());
    }
}

#[test]
fn reference_passes_must_agree_on_tokens_argmax_and_bytes() {
    for pointer in [
        "/execution/passes/1/token_ids/31",
        "/execution/passes/1/argmax_token_ids/31",
        "/execution/passes/0/argmax_token_ids/0",
    ] {
        let mut value = document();
        *value.pointer_mut(pointer).unwrap() = serde_json::json!(999);
        assert!(parse(value, 2).is_err());
    }
    let mut value = document();
    value["execution"]["passes"][1]["decoded_new_tokens"] = serde_json::json!(" Paris. altered");
    assert!(parse(value, 2).is_err());
    for count in [0, 1, 3] {
        let mut value = document();
        let pass = value["execution"]["passes"][0].clone();
        value["execution"]["passes"] = serde_json::Value::Array(vec![pass; count]);
        assert!(parse(value, 2).is_err());
    }
}

#[test]
fn output_bounds_and_the_frozen_prefix_are_checked_before_slicing() {
    for tokens in [vec![], vec![12095], vec![0; 32], vec![151_936; 32]] {
        let mut value = document();
        for pass in value["execution"]["passes"].as_array_mut().unwrap() {
            pass["token_ids"] = serde_json::json!(tokens);
            pass["argmax_token_ids"] = serde_json::json!(tokens);
        }
        assert!(parse(value, 2).is_err());
    }
}

#[test]
fn admission_binding_and_every_generated_output_are_checked() {
    let reference = parse(document(), 2).unwrap();
    assert!(reference.bind("fixture-bundle", &PROMPT_TOKENS).is_ok());
    assert!(reference.bind("other-bundle", &PROMPT_TOKENS).is_err());
    assert!(reference.bind("fixture-bundle", &[]).is_err());
    assert!(reference.check_token(0, 12095).is_ok());
    assert!(reference.check_token(1, 13).is_ok());
    assert!(reference.check_token(2, 0).is_err());
    assert!(reference.check_token(1, 0).is_err());
    assert!(reference.check_output(&PREFIX_TOKENS, PREFIX_BYTES).is_ok());
    assert!(reference.check_output(&[12095, 0], PREFIX_BYTES).is_err());
    assert!(reference.check_output(&PREFIX_TOKENS, b"Paris.").is_err());
}

#[test]
fn profile_arguments_require_exact_identity_and_target_only_tp1() {
    let path = Some(Path::new("reference.json"));
    assert_eq!(
        validate_profile_options(None, None, 8, "legacy prompt", 256, false),
        Ok(false)
    );
    for tokens in [2, 32] {
        assert_eq!(
            validate_profile_options(path, Some(REFERENCE_SHA256), 1, PROMPT, tokens, true),
            Ok(true)
        );
    }
    for (path, digest, world, prompt, count, runtime) in [
        (None, None, 1, PROMPT, 2, true),
        (path, None, 1, PROMPT, 2, false),
        (None, Some(REFERENCE_SHA256), 1, PROMPT, 2, false),
        (path, Some("bad-digest"), 1, PROMPT, 2, false),
        (path, Some(REFERENCE_SHA256), 2, PROMPT, 2, false),
        (path, Some(REFERENCE_SHA256), 1, "Other", 2, false),
        (path, Some(REFERENCE_SHA256), 1, PROMPT, 4, false),
    ] {
        assert!(validate_profile_options(path, digest, world, prompt, count, runtime).is_err());
    }
}

#[test]
fn unpinned_digest_and_nonfile_are_rejected_before_reference_parsing() {
    assert!(Reference::open(Path::new("/does/not/exist"), "bad", 2).is_err());
    assert!(Reference::open(Path::new("/dev/null"), REFERENCE_SHA256, 2).is_err());
}
