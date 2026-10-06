use super::*;
pub(crate) fn bootstrap(mode:InputMode)->Bootstrap {
    Bootstrap { schema:SCHEMA.into(), decode:base::tests::bootstrap(mode),
        projection_image:part(&[10]),guarded_image:Part {
            bytes:28_440,sha256:GUARDED_IMAGE,
        } }
}
pub(crate) fn request(b:&Bootstrap,position:u32,previous:Option<u32>)->Request {
    let mut r=base::tests::request(&b.decode,position,previous);
    r.profile_sha256=b.sha256().unwrap(); r
}
pub(crate) fn control(generation:u64)->Control {
    let old=base::tests::control();
    Control { embedding_ns:old.embedding_ns,tail_ns:old.tail_ns,
        layers:(0..36).map(|i|LayerObservation {
            prefix_states:old.layers[i].prefix_states,mlp_prefixes:old.layers[i].tiles_states,
            guards:[[((generation-1)/2+1) as u32,0,1,0];2],
            prefix_host_ns:[u64::MAX-i as u64,i as u64],segment_host_ns:u64::MAX-100-i as u64,
            observed_queue_frontiers:[((generation-1)*1000+i as u64*10+10,
                (generation-1)*1000+i as u64*10+8);2],
        }).collect() }
}
fn completed(b:&Bootstrap)->(Response,Control,Vec<u8>) {
    let c=control(1);
    let mut bytes=vec![0;OBSERVATION_BYTES];
    bytes[37*8192+6..37*8192+8].copy_from_slice(&0x3f80u16.to_le_bytes());
    let mut value=Completion { generation:1,position:0,input_token:b.decode.input_tokens[0],output_token:3,
        control:part(&c.encode()),observation:part(&bytes),capture:Payload::from_bytes(&bytes).unwrap(),
        chain:[0;32] };
    value.chain=Chain::new(b.decode.registration,b.sha256().unwrap()).advance(&value);
    (Response { schema:RESPONSE_SCHEMA.into(),protocol:PROTOCOL,id:1,device_ids:b.decode.device_ids,
        session:b.decode.scope.session,registration:b.decode.registration,profile_sha256:b.sha256().unwrap(),
        event:Event::Completed(value),native_closed:false,gpu_execution:true,numerical_acceptance:false,
        full_model_acceptance:false,performance_claim:false,production_authority:false },c,bytes)
}
#[test]
fn guarded_wire_profile_is_closed_and_separate_from_every_legacy_recipe() {
    for mode in [InputMode::TeacherForced,InputMode::Autoregressive] {
        let b=bootstrap(mode);
        b.validate(b.decode.device_ids,100,std::process::id(),mode).unwrap();
        assert_ne!(b.sha256().unwrap(),b.decode.sha256().unwrap());
        let initial=b.sha256().unwrap();
        for edit in [
            |b:&mut Bootstrap|b.decode.scope.session[0]^=1,
            |b:&mut Bootstrap|b.decode.scope.group_id+=1,
            |b:&mut Bootstrap|b.projection_image.sha256[0]^=1,
            |b:&mut Bootstrap|b.decode.input_tokens[0]+=1,
        ] {
            let mut changed=b.clone(); edit(&mut changed);
            if let Ok(hash)=changed.sha256() { assert_ne!(hash,initial); }
        }
        for edit in [
            |b:&mut Bootstrap|b.schema.push('x'),
            |b:&mut Bootstrap|b.guarded_image.sha256[0]^=1,
            |b:&mut Bootstrap|b.projection_image.bytes=0,
            |b:&mut Bootstrap|b.decode.begin.tail_image=None,
            |b:&mut Bootstrap|b.guarded_image.bytes=MAX_IMAGE_BYTES as u32+1,
            |b:&mut Bootstrap|b.decode.begin.prefix_image.bytes=u32::MAX,
        ] {
            let mut changed=b.clone();edit(&mut changed);assert!(changed.sha256().is_err());
        }
        let mut json=serde_json::to_value(&b).unwrap();
        json["legacy_fallback"]=serde_json::json!(true);
        assert!(serde_json::from_value::<Bootstrap>(json).is_err());
        assert!(b.validate([12,11],100,std::process::id(),mode).is_err());
        assert!(b.validate(b.decode.device_ids,101,std::process::id(),mode).is_err());
    }
}
#[test]
fn guarded_wire_authenticates_four_image_bodies_and_exact_begin_before_use() {
    let b=bootstrap(InputMode::TeacherForced);
    let mut raw=Vec::new();
    write_header(&mut raw,&mut FrameBudget::new(),&b).unwrap();
    raw.extend_from_slice(&[8,9,10]);
    raw.extend_from_slice(&vec![0;b.guarded_image.bytes as usize]);
    assert!(read_bootstrap(&mut &raw[..],&mut FrameBudget::new()).is_err());
    assert!(write_bootstrap(&mut Vec::new(),&mut FrameBudget::new(),&b,[&[8],&[9],&[10],&[0]]).is_err());
    let request=setup::Request {protocol:setup::PROTOCOL,id:1,device_ids:b.decode.device_ids,
        session:b.decode.scope.session,command:setup::Command::Begin(b.decode.begin.clone())};
    let mut raw=Vec::new();
    setup::write_request(&mut raw,&request,&[7;7]).unwrap();
    let mut incoming=FrameBudget::new();
    assert_eq!(read_begin(&mut &raw[..],&mut incoming,&b).unwrap().1,[7;7]);
    let mut accounted=FrameBudget::new();account_begin(&mut accounted,&b).unwrap();
    assert_eq!(incoming.used(),accounted.used());
    let mut wrong=request;wrong.id=2;
    let mut raw=Vec::new();write_header(&mut raw,&mut FrameBudget::new(),&wrong).unwrap();
    assert!(read_begin(&mut &raw[..],&mut FrameBudget::new(),&b).is_err());
}
#[test]
fn guarded_wire_preserves_all_control_words_u64_timings_and_local_generations() {
    assert!(core::mem::size_of::<Control>() <= 128);
    for generation in 1..=4 {
        let c=control(generation);c.validate(generation).unwrap();
        let mut short=c.clone();short.layers.pop();
        assert!(short.validate(generation).is_err());
        let mut long=c.clone();long.layers.push(c.layers[0].clone());
        assert!(long.validate(generation).is_err());
        let bytes=c.encode();assert_eq!(bytes.len(),CONTROL_BYTES);
        assert_eq!(Control::decode(&bytes,generation).unwrap(),c);
        assert!(Control::decode(&bytes[..bytes.len()-1],generation).is_err());
        assert!(Control::decode(&[bytes.as_slice(),&[0]].concat(),generation).is_err());
        assert!(Control::decode(&bytes,if generation<=2 {3}else{1}).is_err());
        for layer in 0..36 { for rank in 0..2 {
            for edit in 0..5 {
                let mut bad=c.clone();
                match edit {
                    0=>bad.layers[layer].prefix_states[rank][154]=63,
                    1=>bad.layers[layer].mlp_prefixes[rank][290]=63,
                    2=>bad.layers[layer].guards[rank][2]=0,
                    3=>bad.layers[layer].observed_queue_frontiers[rank]=(0,1),
                    _=>bad.layers[layer].guards[rank][3]=1,
                }
                assert!(bad.validate(generation).is_err());
            }
        }}
    }
    assert!(control(1).validate(0).is_err());
    assert!(control(1).validate(5).is_err());
}
#[test]
fn guarded_wire_response_refuses_legacy_schema_claims_nonfinite_and_wrong_argmax() {
    let b=bootstrap(InputMode::TeacherForced);
    let (reply,c,bytes)=completed(&b);
    let mut raw=Vec::new();
    write_response(&mut raw,&mut FrameBudget::new(),&reply,Some(&c),&bytes).unwrap();
    let got=read_response(&mut &raw[..],&mut FrameBudget::new()).unwrap().unwrap();
    assert_eq!(got,(reply.clone(),Some(c.clone()),bytes.clone()));
    for edit in [
        |r:&mut Response|r.schema="FerricPrefixDecodeObservationV1".into(),
        |r:&mut Response|r.full_model_acceptance=true,
        |r:&mut Response|r.numerical_acceptance=true,
        |r:&mut Response|r.performance_claim=true,
        |r:&mut Response|r.production_authority=true,
        |r:&mut Response|r.native_closed=true,
        |r:&mut Response|if let Event::Completed(c)=&mut r.event {c.output_token=4},
    ] {
        let mut bad=reply.clone();edit(&mut bad);
        assert!(write_response(&mut Vec::new(),&mut FrameBudget::new(),&bad,Some(&c),&bytes).is_err());
    }
    let mut badbytes=bytes.clone();badbytes[0..2].copy_from_slice(&0x7fc1u16.to_le_bytes());
    let mut bad=reply.clone();
    if let Event::Completed(v)=&mut bad.event {v.observation=part(&badbytes);v.capture=Payload::from_bytes(&badbytes).unwrap();}
    assert!(write_response(&mut Vec::new(),&mut FrameBudget::new(),&bad,Some(&c),&badbytes).is_err());
    let legacy=base::tests::completed(&base::tests::request(&b.decode,0,None),
        &mut base::Chain::new(b.decode.registration,b.decode.sha256().unwrap()),3).0;
    assert!(serde_json::from_value::<Response>(serde_json::to_value(legacy).unwrap()).is_err());
}
#[test]
fn guarded_wire_request_scope_and_transcript_cannot_relabel_old_execution() {
    let b=bootstrap(InputMode::TeacherForced);
    let r=request(&b,0,None);
    let mut raw=Vec::new();write_request(&mut raw,&mut FrameBudget::new(),&r).unwrap();
    assert_eq!(read_request(&mut &raw[..],&mut FrameBudget::new()).unwrap(),Some(r));
    let (reply,_,_)=completed(&b);
    let Event::Completed(c)=reply.event else {panic!("fixture")};
    let mut old=base::Chain::new(b.decode.registration,b.sha256().unwrap());
    let legacy=base::Completion {generation:c.generation,position:c.position,input_token:c.input_token,
        output_token:c.output_token,control:c.control,observation:c.observation,capture:c.capture.clone(),chain:[0;32]};
    assert_ne!(old.advance(&legacy),c.chain);
    let mut next=c.clone();next.control.sha256[0]^=1;
    assert_ne!(Chain::new(b.decode.registration,b.sha256().unwrap()).advance(&next),c.chain);
}
