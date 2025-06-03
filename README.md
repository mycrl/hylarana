<!--lint disable no-literal-urls-->
<div align="center">
    <img src="./logo.png" width="50px"/>
    <br>
    <br>
    <h1>Hylarana</h1>
</div>
<div align="center">
    <strong>A cross-platform screen cast implemented by Rust.</strong>
</div>
<div align="center">
    <img src="https://img.shields.io/github/actions/workflow/status/mycrl/hylarana/release.yml"/>
    <img src="https://img.shields.io/github/license/mycrl/hylarana"/>
    <img src="https://img.shields.io/github/issues/mycrl/hylarana"/>
    <img src="https://img.shields.io/github/stars/mycrl/hylarana"/>
</div>
<br/>
<br/>
<div align="center">
    <img src="./app.png"/>
    <span>This is a screen cast application implemented using the hylarana SDK.</span>
</div>
<div align="center">
    <span>Watch it in action on YouTube:</span>
    <a href="https://youtu.be/AkW3eRlKl1U">link</a>
</div>
<br/>

## Introduction

This project is both an application and a library, with the application relying on the core library to implement screen mirroring functionality.

Unlike Miracast, AirPlay, and other implementations that depend on hardware support (Wi-Fi Direct), this project can run on most common hardware.

The project is cross-platform, but prioritizes support for Windows, Android, and macOS platforms, with Linux currently only supporting reception. Unlike solutions like DLNA, this project is more akin to AirPlay, with low latency as the primary goal. Currently, latency is maintained at approximately 80-250 milliseconds (with variations depending on the platform and codec used), and it features a highly user-friendly API with minimal external dependencies.

## Technical overview

#### capture

On Android, use a virtual display; on Windows, use WGC; on macOS, use Screen Capture Kit. These capture and screen recording methods are all low-overhead and high-performance, and output hardware-accelerated textures.

#### encoding and decoding

The video uses HEVC, and the audio uses Opus. Hardware-accelerated encoding and decoding are supported on Windows, Android, and macOS. On Windows, you can choose between Intel QSV and D3D11VA. On macOS, Video Toolbox is always used. For Android, support has been implemented for Qualcomm, Kirin, and Rockchip.

#### transmission

The transport layer uses the SRT protocol, configured in low-latency mode. In this mode, SRT acts as a low-latency, semi-reliable transport layer, discarding any packets that exceed the set latency threshold. Although this project is designed to operate exclusively within a pure internal network environment, using SRT improves transmission stability in high-packet-loss network environments such as Wi-Fi.

#### rendering

Video rendering uses WebGPU, a cross-platform HAL layer. Video frame rendering is fully hardware-accelerated, directly rendering GPU textures from each platform to the window via an adaptation layer. This is a high-performance, low-overhead rendering method.

#### hardware acceleration

This project has basically achieved full hardware acceleration across all platforms. Typically, the capture-encoding-decoding-rendering process is fully hardware-accelerated, with video frames only passing between the hardware and GPU. However, this project also takes into account situations where hardware acceleration is not possible. In such cases, software textures are first swapped to the hardware texture buffer before being processed.

## Project structure

-   [android](./android) - The SDK provided for Android use is a Native Module implemented using Kotlin.
-   [applications/app](./applications/app) - Directly use CEF and winit to create desktop applications. This is not Electron, nor is it Tauri.
-   [applications/android](./applications/android) - Android app, UI implemented using WebView, and shares the WebView implementation with the desktop app.
-   [capture](./capture) - Cross-platform screen/audio capture implementation, but no Linux support.
-   [codec](./codec) - Codec implementation that handles HEVC and Opus.
-   [common](./common) - The public section, which contains public types, runtime, atomic operations, strings, logging, platform API wrappers, and more.
-   [discovery](./discovery) - Local area network discovery implemented using UDP broadcast.
-   [hylarana](./hylarana) - Core library implementation, desktop applications are based on this library implementation.
-   [renderer](./renderer) - Cross-platform graphics renderer responsible for rendering video frames to the window.
-   [resample](./resample) - Resampling module, responsible for resampling audio, as well as scaling and converting texture formats using D3D11.
-   [transport](./transport) - The transport layer encapsulates the SRT transport protocol and implements key frame and packet loss handling for audio and video streams.

## Build Instructions

> ❗ This project has a dependency on ffmpeg version 7.1, because it may need to use gpl or nofree dependencies, so this project is not statically linked to ffmpeg. In this case, you need to manually add the dll or so to the dynamic library lookup path, and you can download the ffmpeg build you need at this [link](https://github.com/mycrl/ffmpeg-rs/releases).

#### Requirements

-   [Git](https://git-scm.com/downloads)
-   [Rust](https://www.rust-lang.org/tools/install): Rust stable toolchain.
-   C++20 or above compliant compiler. (G++/Clang/MSVC)
-   [CMake](https://cmake.org/download/): CMake 3.16 or above as a build system.
-   [Node.js](https://nodejs.org/en/download): Node.js 16 or above as a auto build script.
-   [Python3](https://www.python.org/downloads/): Python 3 is required to use the Android Studio Project.

##### Linux (Ubuntu/Debian)

> For Linux, you need to install additional dependencies to build SRT and other.

```bash
sudo apt-get update
sudo apt-get install unzip tclsh pkg-config cmake libssl-dev build-essential libasound2-dev
```

##### Macos

```bash
brew install cmake ffmpeg@7 wget
```

---

#### Build App

```bash
yarn
yarn build:app:release
```

The build product is under the `target/app` directory.

## License

[MIT](./LICENSE) Copyright (c) 2024 mycrl.
