# Redragon K617 vendor HID protocol

This document records what `redragon-k617` actually sends. Everything was
recovered from the official Windows app (`KeyboardDrv.exe`, version 1.6.6) and
verified against a real K617 (`258a:0049`).

## Device and interface

- USB ID: `258a:0049` (BY Tech, Sinowealth SH68F90A / BYK916).
- Interface 1 is the vendor control interface:
  - HID usage page `0xFF00`, usage `0x0001`.
  - Interface 0 is the boot keyboard and does not accept vendor feature
    reports.
- On Linux the interface is a `/dev/hidrawN` node. The rule in
  `udev/99-redragon-k617.rules` grants the logged-in user access.

Report descriptor excerpts for interface 1:

```text
06 00 ff 09 01 a1 01 85 03 ... 81 02      report 3: 3-byte input
06 00 ff 09 01 a1 01 85 05 ... b1 02      report 5: 5-byte feature
06 00 ff 09 01 a1 01 85 06 ... b1 02      report 6: 1031-byte feature
06 00 ff 09 01 a1 01 85 08 ... b1 02      report 8: 381-byte feature
```

The lengths above are payload lengths. A hidraw feature request always
includes the report ID, so the buffers are 6, 1032 and 382 bytes.

## Short command channel (report 5)

### Handshake / read committed state

Write:

```text
05 83 b6 00 00 00
```

Then read report 6 (1032 bytes). The response is the committed state block:

```text
06 83 b6 00 00 00 00 00 ... 5a a5 ... 
```

- Byte 21 is the current firmware effect index.
- Bytes 69 and 71 hold the packed speed/brightness value.
- Bytes 14 and 15 are the `5a a5` marker.

### Read password

Write:

```text
05 81 00 00 00 00
```

Then read report 5 with an 8-byte buffer. Response:

```text
05 01 p0 p1 p2 p3 p4 p5
```

The six password bytes are at offsets 2..8. On the tested K617 they are
`02 00 00 00 00 1a`. The Windows app compares this with `PSD` from `Cfg.ini`.

## Data blocks (report 6)

Every block is 1032 bytes:

```text
byte 0      report ID (0x06)
bytes 1..7  block header
bytes 8..   payload
```

The header is `opcode, cursor_lo, cursor_hi, length_lo, 0, 0, 0`. The vendor
software uses these headers:

| Header bytes 1..7 | Meaning |
| --- | --- |
| `08 b8 00 40 00 00 00` | profile/effect data |
| `09 xx 00 40 00 00 00` | LED matrix write |
| `05 xx 00 40 00 00 00` | key remap matrix |
| `04 d4 00 40 00 00 00` | settings matrix |
| `03 b6 00 00 00 00 00` | commit (136-byte config) |

The profile block copies 1024 bytes of effect data starting at report offset 8.
Only the first 1003 bytes are accepted by the Windows app (`length <= 0x3eb`).
The base colour is at payload offsets 21..23, i.e. report offsets 29..31.

## Effect modes

The firmware effect index is one of:

| # | Name | # | Name |
| --- | --- | --- | --- |
| 1 | Fixed on | 12 | Reaction |
| 2 | Respire | 13 | Sine wave |
| 3 | Rainbow | 14 | Retinue scanning |
| 4 | Flash away | 15 | Rotating windmill |
| 5 | Raindrops | 16 | Colorful waterfall |
| 6 | Rainbow wheel | 17 | Blossoming |
| 7 | Ripples shining | 18 | Rotating storm |
| 8 | Stars twinkle | 19 | Collision |
| 9 | Shadow disappear | 20 | Perfect |
| 10 | Retro snake | 21 | Self-define |
| 11 | Neon stream | 22 | Off |

The indices match `tc_led_mode1` .. `tc_led_mode22` in the vendor
`Text/en/text.xml`.

## Speed and brightness

The commit block stores a packed byte at offsets 69 and 71:

```text
high nibble = speed      (1..4 in the UI)
low  nibble = brightness (1..4 in the UI)
```

Example: speed 4, brightness 2 -> `0x42`.

## Per-key frame (report 8)

The direct per-key frame is the Sinodragon format:

```text
byte 0      report ID 0x08
bytes 1..3  0a 7a 01
bytes 4..   96 RGB triplets, column-major 16x6 matrix
rest        zero padding to 382 bytes
```

Slot positions used by the K617 (verified anchors: Escape=0, Tab=2,
CapsLock=3, LShift=4, LCtrl=5, J=45, Space=35, Enter=81):

| Row | Keys and slots |
| --- | --- |
| 0 | Esc 0, 1 7, 2 13, 3 19, 4 25, 5 31, 6 37, 7 43, 8 49, 9 55, 0 61, - 67, = 73, Bksp 79 |
| 1 | Tab 2, Q 8, W 14, E 20, R 26, T 32, Y 38, U 44, I 50, O 56, P 62, [ 68, ] 74, \ 80 |
| 2 | Caps 3, A 9, S 15, D 21, F 27, G 33, H 39, J 45, K 51, L 57, ; 63, ' 69, Enter 81 |
| 3 | LShift 4, Z 10, X 16, C 22, V 28, B 34, N 40, M 46, , 52, . 58, / 64, RShift 82 |
| 4 | LCtrl 5, LWin 11, LAlt 17, Space 35, RAlt 53, Fn 59, Menu 65, RCtrl 71 |

Slots that do not exist on the K617 stay black.

## How the frames were recovered

The Windows packet builders are pure once `memset`, `memcpy`, the security
cookie and `HidD_SetFeature` are stubbed. The emulation harness in
`analysis/emu_builders.py` (next to the crate in the working tree) maps the
executable at its preferred base and runs the builders directly with Unicorn.
This produced the exact buffer layouts above, including the fact that the
Windows app passes 9 bytes to report 5 and that hidraw must not do that.

## Known gaps

- **Direction.** The vendor app has a per-effect direction control. The commit
  block contains a parameter table whose first byte per entry looks like a
  direction/group selector, but the exact mapping was not confirmed on
  hardware, so the CLI does not expose it yet.
- **Key remapping and macros.** The frames are known (`05 xx ...` for the key
  matrix and `84 d4` on report 5 for macro data), but the 1024-byte matrix is
  produced by a byte-swapping document model that depends on the vendor app's
  in-memory key table. Reimplementing that model is a separate project.
- **Vendor `.pro` profile files.** The JSON profiles are a deliberate
  reimplementation, not a decoder for the proprietary format.
- **Firmware update.** `Update.exe` is not used.
