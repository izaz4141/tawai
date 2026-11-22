# Glossary

Canonical metadata vocabulary for the audio tag model and the tag editor. Terms
here are load-bearing: they disambiguate fields that are easy to confuse.

## Release date vs Original date

- **Release date** (`release_date`, lofty `tag.date()`) — the date of *this*
  release.
- **Original date** (`original_date`, `ItemKey::OriginalReleaseDate`) — the date
  of the work's first release.

## Media

`media` = `ItemKey::OriginalMediaType` — the release medium (CD, vinyl,
digital). Not the file's container or codec.

## Record label

`record_label` = `ItemKey::Label` (ID3 `TPUB`). The publisher/organization for
the release.

## Encoder

`encoder` = `ItemKey::EncodedBy` — the software or person that encoded the file.
Read-only in the tag editor.

## Artist, track artists, album artists

- An **artist** is a single entity, stored once in the `artists` table.
- **Track artists** (`artists`) and **album artists** (`album_artists`) are
  role-specific associations to that shared entity, not separate entities.
- The tag editor keeps them as two distinct fields so they can be written to
  their distinct keys (`TrackArtists`, `AlbumArtists`). For display, both are
  derived from the corresponding singular strings via `parse_artists`.

## Length

**Length** is the file's `duration_secs` (a file property, not a tag).
Read-only.

## Read-only fields

File path, length, encoder, ReplayGain track gain, ReplayGain track peak,
original year.

## Read set vs write set

The tag editor sends the complete editable set on every write
(`EditableTagData` / `writeFileTags`), and `TagWritePolicy::replace()` writes
every field, clearing stored values for fields the user emptied. Cover is
treated as write-if-present in replace mode unless explicitly cleared;
read-only fields are never sent or written; release status is not read because
no `ItemKey` exists for it.

## Security

**Master key**:
32 random bytes, presented to the operator as 64 hex characters, that encrypt
stored API keys and sign JWTs. Created once, supplied through
`TAWAI_SERVER_MASTER_KEY`, and never sent to a client afterwards.
_Avoid_: encryption key, root key, secret key, master password

**API key**:
A per-user credential issued by the server, stored encrypted under the master
key, and presented by clients in `X-API-Key`.
_Avoid_: token, access key, password

**JWT**:
A short-lived session credential signed with the master key, carried either in
the `tawai_jwt` cookie or as the `token` query parameter on streaming endpoints.
_Avoid_: session token, bearer, access token

**Redacted value**:
A value that serialises normally but whose `Debug` output never contains it.
Protects against accidental logging only; the value is fully readable in memory.
_Avoid_: secret, protected value

**Protected secret**:
A value held in memory that can only be read inside an explicit borrow scope,
and is zeroed when freed. Stronger than a redacted value, and only possible for
values that never leave the process.
_Avoid_: secret, encrypted value
