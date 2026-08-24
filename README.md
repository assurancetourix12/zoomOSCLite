# ZoomOSC Lite

ZoomOSC Lite is a small open-source OSC remote for the Zoom Workplace client on
macOS. It lets an OSC controller such as Bitfocus Companion control camera
sharing, microphone mute, video, and Zoom audio profiles.

It does not contain Zoom code or use the Zoom SDK. It controls the official Zoom
client through the macOS Accessibility API.

## Requirements

- macOS 13 or later on Apple Silicon
- The Zoom Workplace desktop client
- A Zoom meeting in progress
- macOS Accessibility permission for ZoomOSC Lite

## Install and run

1. Download the latest ZIP from
   [GitHub Releases](https://github.com/assurancetourix12/zoomOSCLite/releases).
2. Extract it and move **ZoomOSC Lite.app** to `/Applications`.
3. Open ZoomOSC Lite.
4. Open **System Settings → Privacy & Security → Accessibility** and enable
   **ZoomOSC Lite**.
5. Return to ZoomOSC Lite and start or restart the OSC server.

The application starts with these safe defaults:

- **This Mac only** (`Apenas este Mac`): `127.0.0.1`
- **UDP port:** `9000`

Select **Local network** (`Rede local`) when Companion is running on another
computer or device. Click **Apply and restart** (`Aplicar e reiniciar`) after
changing the access mode or port.

The **Launch at login** (`Iniciar ao entrar no macOS`) option registers the
application with the native macOS login-items service. For reliable login
launching, keep the application in `/Applications`.

> OSC is transported over UDP without authentication. Only enable **Local
> network** on a trusted network, and never expose the UDP port to the Internet.

## Bitfocus Companion setup

ZoomOSC Lite works with Companion's
[**Generic → OSC** connection](https://bitfocus.io/connections/generic-osc). No
dedicated Companion module is required.

### 1. Add the Generic OSC connection

1. Open the Companion web interface.
2. Go to **Connections** and select **Add connection**.
3. Search for **OSC** and add **Generic → OSC**.
4. Give the connection a useful name, such as `ZoomOSC Lite`.
5. Set the target IP:
   - If Companion and ZoomOSC Lite run on the same Mac, use `127.0.0.1` and
     leave ZoomOSC Lite in **This Mac only** mode.
   - If Companion runs on another device, enter the IP address of the Mac
     running ZoomOSC Lite and select **Local network** in ZoomOSC Lite.
6. Set the target port to `9000`, or to the custom UDP port shown in ZoomOSC
   Lite.
7. Use **UDP** as the transport, then save the connection.

Because OSC over UDP is connectionless, Companion may not be able to prove that
ZoomOSC Lite is reachable. Check the **Last message** field in ZoomOSC Lite after
pressing a test button.

### 2. Add an action to a Companion button

1. Open **Buttons** and select the button you want to configure.
2. In the button's **Actions** section, add a **press/down** action.
3. Select the `ZoomOSC Lite` Generic OSC connection.
4. Choose **Send message without arguments**.
5. Paste one of the OSC paths from the table below into the **OSC path** field.
6. Save the button and press it while a Zoom meeting is open.

The commands do not need an OSC value or argument. For example, a microphone
mute button only needs this path:

```text
/zoom/audio/mute
```

For separate ON and OFF controls, create separate Companion buttons and assign
the corresponding absolute command to each one. Alternatively, add different
commands to the button's press and release actions if that suits your workflow.

## OSC commands

| OSC path | Action |
| --- | --- |
| `/zoom/share/camera/start` | Select and share content from the second camera |
| `/zoom/share/stop` | Stop the current share |
| `/zoom/audio/mute` | Mute the microphone if it is currently unmuted |
| `/zoom/audio/unmute` | Unmute the microphone if it is currently muted |
| `/zoom/video/on` | Start video if it is currently off |
| `/zoom/video/off` | Stop video if it is currently on |
| `/zoom/audio/profile/noise-removal` | Select Zoom background-noise removal |
| `/zoom/audio/profile/isolation` | Select personalized audio isolation |
| `/zoom/audio/profile/original` | Select original sound for musicians |
| `/zoom/audio/profile/live-performance` | Select live performance audio |

ZoomOSC-style aliases are also available:

| Alias | Equivalent command |
| --- | --- |
| `/zoom/me/startCameraShare` | `/zoom/share/camera/start` |
| `/zoom/me/stopShare` | `/zoom/share/stop` |
| `/zoom/me/mute` | `/zoom/audio/mute` |
| `/zoom/me/unmute` | `/zoom/audio/unmute` |
| `/zoom/me/startVideo` | `/zoom/video/on` |
| `/zoom/me/stopVideo` | `/zoom/video/off` |

The microphone and video commands are absolute and idempotent: ZoomOSC Lite
checks the current Zoom state and does nothing if the requested state is already
active.

## Troubleshooting

- Make sure a Zoom meeting is open and its window is visible.
- Confirm that ZoomOSC Lite is enabled in macOS Accessibility settings.
- If Companion is on another device, select **Local network** in ZoomOSC Lite
  and use the Mac's LAN IP instead of `127.0.0.1`.
- Confirm that the UDP port is identical in both applications.
- Check **Last message** in ZoomOSC Lite to see whether a command arrived and
  whether it succeeded.
- After replacing the application with a new version, macOS may require you to
  remove and re-enable its Accessibility permission.

For detailed Accessibility diagnostics, run the helper while Zoom is visible:

```sh
zoomosc-lite inspect
```

This prints the control names exposed by the installed Zoom version. It is
useful when a Zoom update or translation changes the interface labels.

## Build from source

Building requires Xcode Command Line Tools, Rust, and
[mise](https://mise.jdx.dev/).

```sh
mise run app
```

The application is created at `dist/ZoomOSC Lite.app`.

To create an ARM64 ZIP and its SHA-256 checksum:

```sh
mise run release
```

The current build uses an ad-hoc signature. Public distribution without macOS
Gatekeeper warnings requires an Apple Developer ID certificate and Apple
notarization.

## Limitations

ZoomOSC Lite relies on the Accessibility interface exposed by Zoom. A future
Zoom interface update may require its selectors to be updated. Common English
and Portuguese labels are currently supported.

## Contributing

Issues and pull requests are welcome. Before submitting changes, run:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

## License

[MIT](LICENSE)
