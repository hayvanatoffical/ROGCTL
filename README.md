# rogctl

**English** | [Türkçe](README.tr.md)

A thermal and power controller for ASUS ROG laptops. It replaces Armoury
Crate's manual mode: it measures what the machine is actually doing, rewrites
the fan curve and the GPU clock ceiling to match, and starts itself at logon.

Developed and measured on a **ROG Strix SCAR 15 (G533ZW, i9-12900H +
RTX 3070 Ti)**. Every default number in here was measured on that machine. It
will run on other ASUS models, but you should retune the numbers with
`rogctl kurulum`.

> The interface is in Turkish — commands, config keys and output. This README
> is the English map to it. Command names are given verbatim below.

## What it does

- **Classifies the workload** (idle / video / office / light game / AAA game /
  render) from CPU load, GPU load, VRAM occupancy and the NVDEC counter.
- **Writes the fan curve** for that mode over ACPI.
- **Caps the GPU clock** to hold a temperature target. Since power ≈ V²·f,
  lowering the clock drops the driver to a lower voltage point — an undervolt
  through a supported API.
- **Limits the frame rate** through the NVIDIA driver profile. A frame nobody
  sees is pure heat; this is the single largest thermal lever available.
- **Trims the standby RAM list** on mode transitions and under real memory
  pressure.
- Behaves differently on AC and on battery; on battery it caps how far the mode
  can escalate.

## Hardware support

| Layer | Requirement |
|---|---|
| Fan curves, power mode, PPT, fan RPM | **ASUS only** (the ATKACPI driver) |
| GPU clock ceiling, GPU telemetry | Any NVIDIA GPU (NVML) |
| Frame limit, VSync, VRR readback | Any NVIDIA GPU (NVAPI) |
| Workload classification, standby RAM, AC/battery detection | Vendor-neutral |

The daemon currently requires ATKACPI at startup, so it **will not start on
non-ASUS hardware**. The NVIDIA-side commands (`rogctl nv`, `rogctl valorant`)
do not have that restriction, but nothing runs automatically without the daemon.

## Install

```powershell
cargo build --release
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

The script re-launches itself elevated if needed — the NVML clock lock can only
be written with administrator rights. Install registers a scheduled task named
`rogctl` that runs elevated at logon, and adds `bin\` to PATH.

Then run the wizard:

```powershell
rogctl kurulum
```

It probes the hardware first (how many ATKACPI devices the BIOS implements,
whether NVML and NVAPI opened, panel refresh rate, VRR state), reports what it
found, then asks four questions: priority (performance / balanced / quiet),
frame limit, standby RAM trimming, and whether to stop the Armoury Crate
services. **Nothing changes without being asked**, and an existing
`rogctl.yaml` is backed up before it is replaced.

To uninstall:

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1 -Uninstall
```

Uninstalling hands the fan curves back to Armoury Crate and releases the GPU
clock lock. Settings written to the **driver profile are persistent** and
uninstall does not touch them — undo those separately with `rogctl nv sifirla`.

## Daily use

Nothing. It starts at logon and runs itself. If you are curious:

```
rogctl status     # what the daemon is doing
rogctl rapor      # build an HTML report from the log and open it
rogctl mon 60     # live telemetry
rogctl kurulum    # re-run the setup wizard
rogctl            # command list
```

## Levers that were TRIED and DO NOT WORK on this machine

These are not assumptions. Each was implemented, tested and rejected. Not worth
retrying:

| Lever | Result |
|---|---|
| CPU power limit (ACPI PPT, PERF_MODE, powercfg PROCTHROTTLEMAX) | Locked by ASUS firmware. The i9-12900H targets Tjmax by design; 95 °C is normal. |
| NVML GPU power limit | `NVML_ERROR_NOT_SUPPORTED` (vBIOS lock) |
| Anything via MSR | Memory Integrity (HVCI) is on, WinRing0 will not load |

What actually works: ACPI fan curves, the NVML clock lock, the NVAPI driver
profile, standby RAM trimming.

**A CPU at 95 °C is not a fault here.** There is no power lever on this machine
that lowers it; the only thing left is moving the heat out faster, which is the
fan curve.

## Frame rate and Valorant

The frame limit is written into the NVIDIA driver profile and survives rogctl
being closed. Which side of the refresh rate it sits on depends on VRR:

- **VRR/G-SYNC on** (the default on this panel) → the limit goes `vrr_margin`
  **below** refresh (144 → 141). A frame above that falls out of the VRR window.
- **VRR off** → the limit goes `fps_headroom` **above** refresh (144 → 151),
  because the driver's limiter lands a few frames under its target.

To see which profile is doing the limiting:

```
rogctl nv kim
```

**Valorant is a special case.** It keeps four separate frame caps in its own
file (`RiotUserSettings.ini`), invisible to the driver — so the game can sit
locked at 60 fps while the machine is configured perfectly at the driver level.
The caps are **per account**; fixing one account does not fix the others.

```
rogctl valorant                 # show the caps for the last account played
rogctl valorant hepsi           # all accounts
rogctl valorant hepsi duzelt    # set them all to the refresh rate
rogctl valorant hepsi serbest   # remove the cap entirely
```

`serbest` also writes `Frame Rate Limiter = 0` into the driver's **Valorant
profile**, on top of clearing the keys in the game's ini. Because an
application profile overrides the base profile, the global limit stays in force
everywhere else. `duzelt` reverses this.

> Run these with Valorant **closed**. On exit the game rewrites the ini from
> memory and silently discards the change; the command already refuses to run
> while the game is up. A `.rogctl-yedek` backup is taken before every write.

## Configuration

`bin\rogctl.yaml`. After editing:

```powershell
Stop-ScheduledTask rogctl ; Start-ScheduledTask rogctl
```

Highlights:

- `modes:` — per mode: fan range, GPU clock ceiling/floor, temperature targets
- `games:` — force a mode while a given exe is running (overrides the classifier)
- `battery:` — fan and clock scaling on battery, plus a mode ceiling
- `nvidia.refresh_hz` — 0 reads the panel live; a number makes the limit always
  derive from that rate (guards against a wrong reading at logon)
- `nvidia.respect_vrr` — set false to apply the +`fps_headroom` rule even with
  VRR on

The file is never overwritten. When a measured default changes, `CONFIG_VERSION`
is bumped and install backs the old file up as `rogctl.yaml.eski` before
regenerating.

## One setting to avoid

Leave `Idle Application Max FPS Limit` (0x10835016) off. The driver's "idle"
decision fires during gameplay too, and pins the display to 30 fps.

## When something goes wrong

```
rogctl status          # is the daemon up, what is it doing
rogctl probe           # which hardware levers can be opened
rogctl selftest        # exercise the write paths
rogctl nv kim          # who is limiting the frame rate
```

Log: `bin\rogctl.log` (rolled to `rogctl.log.1` past 1.5 MB).

## Source layout

| File | Contents |
|---|---|
| `main.rs` | command dispatch, daemon loop, status/report output |
| `kurulum.rs` | setup wizard: hardware probe, questions, config writing |
| `policy.rs` | workload classification, mode envelopes, clock governor |
| `telemetry.rs` | CPU/GPU sampling |
| `acpi.rs`, `control.rs`, `devices.rs` | ACPI fan curves and device writes |
| `gpu.rs` | NVML (clock lock, load, VRAM) |
| `nvapi.rs` | NVIDIA driver profile (DRS) |
| `valorant.rs` | Valorant's own settings file |
| `memory.rs` | standby list trimming |
| `report.rs` | HTML report generation |

`deneysel/` holds an abandoned GUI and installer scaffold. Nothing in it is
wired up or part of the build — see its own README.

## License

See [LICENSE.txt](LICENSE.txt).
