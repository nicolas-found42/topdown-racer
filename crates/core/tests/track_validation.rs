use topdown_racer_core::track::Track;

#[test]
fn duplicate_vertices_are_rejected_with_the_segment_index() {
    for (points, segment) in [
        ("[[0,0],[0,0],[10,0],[10,10],[0,0]]", 0),
        ("[[0,0],[10,0],[10,0],[10,10],[0,0]]", 1),
        ("[[0,0],[10,0],[10,10],[0,0],[0,0]]", 3),
    ] {
        let text =
            format!(r#"{{"name":"Duplicate vertex","width":10,"points":{points},"surfaces":[]}}"#);
        let error = Track::parse(&text).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("track segment {segment} has zero length; consecutive vertices must differ")
        );
    }
}
