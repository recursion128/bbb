use bbb_server_datapack::tag_loader::{parse_tag_file, EntryWithSource, LoadedTagEntries};

pub(crate) fn function_tag_entries(
    registry_id: &str,
    tags: &[(&str, &[&str])],
) -> LoadedTagEntries {
    LoadedTagEntries {
        registry_id: registry_id.to_owned(),
        tags: tags
            .iter()
            .map(|(tag, members)| {
                let values: Vec<_> = members
                    .iter()
                    .map(|member| match member.strip_suffix('?') {
                        Some(id) => serde_json::json!({"id": id, "required": false}),
                        None => serde_json::json!(member),
                    })
                    .collect();
                let file = parse_tag_file(
                    &serde_json::to_vec(&serde_json::json!({"values": values})).unwrap(),
                )
                .unwrap();
                (
                    (*tag).to_owned(),
                    file.entries
                        .into_iter()
                        .map(|entry| EntryWithSource {
                            entry,
                            source: "test".to_owned(),
                        })
                        .collect(),
                )
            })
            .collect(),
    }
}
