# Army Men RTS `.x` Archive Format

This document describes the `.x` archive format, the format identifier stored in the file is `NORK`.

Evidence sources:

- Static analysis of `attic.x' with Ghidra.
- Byte-level inspection of `attic.x`.

That inspected archive is 1,249,561 bytes long and contains 15 members.

## Simple description

An archive consists of:

1. A fixed 56-byte header.
2. Raw member data.
3. A directory at the end describing each member's name, offset, and size.

To extract a member, a reader obtains its directory record, seeks to the
recorded payload offset, and reads the recorded payload size. The packer treats
member data as opaque bytes.

## Numeric representation

The multi-byte numeric values below are unsigned 32-bit little-endian values.
This is directly visible in the archive: version `1` is `01 00 00 00`, the
member count `15` is `0F 00 00 00`, and the file size `1,249,561` is stored as
`19 11 13 00`.

Offsets and sizes are measured in bytes from the beginning of the file. The
first byte is offset `0`.

## Header: offsets `0x00` through `0x37`

The header is exactly 56 bytes.

| Offset | Size | Type | Observed value or meaning |
|---:|---:|---|---|
| `0x00` | 4 | bytes | ASCII signature `NORK` (`4E 4F 52 4B`) |
| `0x04` | 4 | `u32` | Format version; observed value `1` |
| `0x08` | 4 | `u32` | Zero in the inspected archive; meaning not established |
| `0x0C` | 4 | `u32` | Archive byte length; observed value `1,249,561` |
| `0x10` | 4 | `u32` | Directory record count; observed value `15` |
| `0x14` | 4 | `u32` | Directory start offset; observed value `1,249,141` (`0x130F75`) |
| `0x18`–`0x37` | 32 | bytes | Zero in the inspected archive; individual meanings not established |

The reference reader reads 56 header bytes, checks `NORK`, and accepts version
`1`. Runtime mutation tests showed that a different signature is rejected as
an invalid archive and version `2` is rejected as unsupported.

The creator writes the final archive length at `0x0C` and the directory start
at `0x14`. The meanings of the other zero fields are not established.

## Payload area

In `attic.x`, the first payload starts immediately after the header at offset
`0x38` (decimal `56`). Payloads are contiguous in that archive. Each directory
record supplies the authoritative offset and size:

```text
payload end = payload offset + payload size
```

The analyzed creator copies source bytes without an observed compression step.
No compression flag or compression metadata was identified. The format does
not define an encoding for the contents of `.cfg`, `.tga`, `.blk`, or other
members; they are opaque byte sequences to the packer.

## Directory record layout

The directory begins at the header's `0x14` offset. Each record is variable
length:

| Relative position | Size | Type | Meaning |
|---:|---:|---|---|
| `+0x00` | 4 | `u32` | Member field; zero in every inspected record; meaning not established |
| `+0x04` | 1 | `u8` | Name length in bytes |
| `+0x05` | variable | bytes | Name bytes, exactly the preceding length |
| after name | 4 | `u32` | Payload offset |
| after offset | 4 | `u32` | Payload size |

The record length is:

```text
13 + name_length
```

There is no separate serialized name terminator. The parser advances by the
stored length. The analyzed writer rejects names longer than 255 bytes because
the length field is one byte. The inspected names are ASCII; the evidence does
not establish a universal encoding for non-ASCII names. A precise binary
reader should treat the name as a counted byte string and choose its decoding
policy explicitly.

## First two records in `attic.x`

The first record begins at directory offset `1,249,141`:

```text
00 00 00 00    member field
0D            name length = 13
base_army.cfg name bytes
38 00 00 00    payload offset = 56
78 02 00 00    payload size = 632
```

The second record is:

```text
00 00 00 00    member field
16            name length = 22
base_army_orderers.cfg
B0 02 00 00    payload offset = 688
D9 07 00 00    payload size = 2,009
```

The first payload ends at `56 + 632 = 688`, the second payload's offset.

## Complete inspected directory

These values were read directly from `attic.x`. `record field` is the initial
four-byte record field and was zero for every inspected record.

| # | Record field | Name length | Payload offset | Payload size | Name |
|---:|---:|---:|---:|---:|---|
| 1 | 0 | 13 | 56 | 632 | `base_army.cfg` |
| 2 | 0 | 22 | 688 | 2,009 | `base_army_orderers.cfg` |
| 3 | 0 | 14 | 2,697 | 350 | `debriefing.cfg` |
| 4 | 0 | 23 | 3,047 | 786,476 | `engine_loading_back.tga` |
| 5 | 0 | 8 | 789,523 | 2,173 | `game.cfg` |
| 6 | 0 | 20 | 791,696 | 3,190 | `objectives_green.cfg` |
| 7 | 0 | 18 | 794,886 | 4,346 | `objectives_tan.cfg` |
| 8 | 0 | 11 | 799,232 | 75,418 | `objects.cfg` |
| 9 | 0 | 17 | 874,650 | 1,553 | `recruit_types.cfg` |
| 10 | 0 | 13 | 876,203 | 134 | `strategic.cfg` |
| 11 | 0 | 11 | 876,337 | 270,753 | `terrain.blk` |
| 12 | 0 | 11 | 1,147,090 | 49,170 | `terrain.tga` |
| 13 | 0 | 18 | 1,196,260 | 49,196 | `terrain_custom.tga` |
| 14 | 0 | 9 | 1,245,456 | 3,621 | `types.cfg` |
| 15 | 0 | 17 | 1,249,077 | 64 | `types_mission.cfg` |

The directory ends at offset `1,249,561`, equal to both the physical file
length and the stored header file-length field.

## Format facts established by the evidence

- The signature is `NORK`.
- The analyzed reader expects version `1`.
- The header/read size is 56 bytes.
- The header contains a member count and a directory offset.
- Each record contains a four-byte field, one-byte name length, counted name
  bytes, payload offset, and payload size.
- The analyzed creator limits a serialized name length to 255 bytes.
- A zero-byte member is valid; it was created and extracted successfully.
- The creator copies raw member data; no compression behavior was observed.

## Information not established

The following must not be presented as known format semantics:

- The meaning of header bytes at `0x08` or `0x18`–`0x37`.
- The meaning of the four-byte field at the start of a directory record.
- A universal encoding for names outside the inspected ASCII examples.
- A compression scheme or alternate payload encoding.
- Whether every producer always places the first payload at `0x38`; this was
  observed in `attic.x` and produced by the analyzed creator.
- Whether archives beyond the 32-bit offset/size range are supported.
- Whether other signatures or versions belong to other game builds.

The Rust implementation adds safety validation for malformed offsets and safe
extraction paths. 
