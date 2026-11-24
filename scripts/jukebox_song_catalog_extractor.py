"""Tree-sitter-backed extraction for the vanilla jukebox-song catalog."""

from __future__ import annotations

from dataclasses import dataclass

from bootstrap_catalog_extractor import bootstrap_resource_key_entries
from sound_event_catalog_extractor import direct_sound_event_entries
from vanilla_java_source import JavaExpression, JavaSource


class JukeboxSongCatalogExtractionError(ValueError):
    pass


@dataclass(frozen=True)
class JukeboxSongEntry:
    constant_name: str
    resource_id: str
    sound_event_id: str


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    span = expression.span
    raise JukeboxSongCatalogExtractionError(
        f"{source.relative_path}:{span.start_point[0] + 1}:"
        f"{span.start_point[1] + 1}: {message}"
    )


def jukebox_song_entries(
    source: JavaSource,
    sound_events_source: JavaSource,
    expected_count: int = 21,
    expected_sound_event_count: int = 1808,
) -> list[JukeboxSongEntry]:
    resources = bootstrap_resource_key_entries(
        source, "JukeboxSong", {"create"}, expected_count
    )
    methods = [method for method in source.methods if method.name == "bootstrap"]
    if len(methods) != 1 or methods[0].body is None:
        raise JukeboxSongCatalogExtractionError(
            f"{source.relative_path}: expected exactly one bootstrap method with a body"
        )
    body = methods[0].body
    calls = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is None
        and body.start_byte <= call.span.start_byte
        and call.span.end_byte <= body.end_byte
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    if len(calls) != len(resources):
        raise JukeboxSongCatalogExtractionError(
            f"{source.relative_path}: extracted {len(calls)} bootstrap calls for "
            f"{len(resources)} resource keys"
        )

    sound_events = {
        entry.constant_name: entry.resource_id
        for entry in direct_sound_event_entries(
            sound_events_source, expected_sound_event_count
        )
    }
    entries: list[JukeboxSongEntry] = []
    for resource, call in zip(resources, calls):
        if len(call.arguments) != 5:
            _fail(source, source.expression(call.node), "jukebox register arity changed")
        if call.arguments[1].canonical != resource.constant_name:
            _fail(source, call.arguments[1], "jukebox resource/bootstrap order mismatch")
        sound_expression = call.arguments[2]
        if sound_expression.node.type != "field_access":
            _fail(source, call.arguments[2], "jukebox sound is not a SoundEvents constant")
        owner = sound_expression.node.child_by_field_name("object")
        name = sound_expression.node.child_by_field_name("field")
        if owner is None or name is None or source.text(owner) != "SoundEvents":
            _fail(source, sound_expression, "jukebox sound is not a SoundEvents constant")
        sound_event_id = sound_events.get(source.text(name))
        if sound_event_id is None:
            _fail(source, call.arguments[2], "jukebox sound constant does not exist")
        entries.append(
            JukeboxSongEntry(
                resource.constant_name, resource.resource_id, sound_event_id
            )
        )

    if len({entry.resource_id for entry in entries}) != len(entries):
        raise JukeboxSongCatalogExtractionError(
            f"{source.relative_path}: duplicate jukebox song resource id"
        )
    if len({entry.sound_event_id for entry in entries}) != len(entries):
        raise JukeboxSongCatalogExtractionError(
            f"{source.relative_path}: duplicate jukebox song sound event id"
        )
    return entries
