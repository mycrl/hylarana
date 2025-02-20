<!--lint disable no-literal-urls-->
<div align="center">
   <img src="./logo.png" width="50px"/>
   <br>
   <br>
   <br>
   <h1>Hylarana</h1>
</div>
<br/>
<div align="center">
  <strong>A cross-platform screen cast implemented by Rust.</strong>
</div>
<div align="center">
  <img src="https://img.shields.io/github/actions/workflow/status/mycrl/hylarana/release.yml"/>
  <img src="https://img.shields.io/github/license/mycrl/hylarana"/>
  <img src="https://img.shields.io/github/issues/mycrl/hylarana"/>
  <img src="https://img.shields.io/github/stars/mycrl/hylarana"/>
</div>

<div align="center">
  <span>documentation:</span>
  <a href="https://docs.rs/hylarana/latest/hylarana">docs.rs</a>
</div>
<div align="center">
  <span>examples:</span>
  <a href="./examples/rust">rust</a>
  <span>/</span>
  <a href="./examples/Android">kotlin</a>
</div>
<div align="center">
  <span>watch the demo on youtube:</span>
  <a href="https://youtu.be/AkW3eRlKl1U">link</a>
</div>
<br/>
<br/>

---

Unlike implementations such as Miracast, AirPlay, etc. that rely on hardware support (WIFI Direct), this library works on most common hardware.

The project is cross-platform, but the prioritized supported platforms are Windows, Android, Macos, with Linux only supported for reception. Unlike solutions such as DLNA, this project is more similar to airplay, so low latency is the main goal, currently the latency is controlled at around 80-250ms (it will be different on different platforms with different codecs), and maintains a very easy to use API and few external dependencies.

Unlike traditional screen casting implementations, this project can work in forwarding mode, in which it can support casting to hundreds or thousands of devices at the same time, which can be useful in some specific scenarios (e.g., all advertising screens in a building).

## How was this achieved?

The first is screen capture, this part of each platform independently separate implementation, but all use hardware accelerated texture, Android use virtual display, Windows use WGC, and Macos use screenshencapturekit.

In terms of audio and video codecs, H264 is used for video and Opus is used for audio. Similarly, Windows, Android and Macos all provide hardware accelerated codecs, and the current hardware codecs on Windows are adapted to Qsv and D3D11VA, Android is adapted to Qualcomm, Kirin, and RK series of socs, while Macos uses the Video Toolbox.

Both SRT and UDP multicast schemes are used for the transport layer of the data. The audio and video data transmitted by the transport layer are bare streams and do not contain similar encapsulations such as FLV. For SRT, many parameters have been adjusted in detail to suit the LAN environment, so that when using the SRT transport layer, the delay can be controlled at about 20-40 ms. The UDP multicast scheme has only a receive buffer and no transmit buffer, and the fixed maximum delay of UDP multicast is 40 ms, which is used to sort and wait for packets in the buffer.

The graphics interface also uses two solutions, Direct3D11 and WebGPU. WebGPU is a cross-platform graphics interface wrapper library, but WebGPU can't work on some old devices on Windows, because WebGPU needs at least Direct3D12 support, so Direct3D11 is provided on Windows. Similarly, the graphics implementations for Windows, Android, Macos are all fully hardware accelerated. In general, the capture, encoding, decoding and display of a video frame is performed inside the GPU, and the scaling and formatting of video frames on Windows is also fully hardware accelerated. For Macos and Android the situation is somewhat less so, except for YUV textures which are not available hardware accelerated, otherwise in line with Windows, they are fully hardware accelerated.

## Build Instructions

#### Requirements

-   [Git](https://git-scm.com/downloads)
-   [Rust](https://www.rust-lang.org/tools/install): Rust stable toolchain.
-   C++20 or above compliant compiler. (G++/Clang/MSVC)
-   [CMake](https://cmake.org/download/): CMake 3.16 or above as a build system.
-   [Node.js](https://nodejs.org/en/download): Node.js 16 or above as a auto build script.
-   [Cargo NDK](https://github.com/willir/cargo-ndk-Android-gradle): Cargo NDK is optional and required for Android Studio projects.

##### Linux (Ubuntu/Debian)

> For Linux, you need to install additional dependencies to build SRT and other.

```sh
sudo apt-get update
sudo apt-get install tclsh pkg-config cmake libssl-dev build-essential libasound2-dev libsdl2-dev libva-dev v4l-utils
```

##### Macos

```sh
brew install cmake ffmpeg@7
```

---

#### Build

Examples and SDK library files can be automatically packaged by running an automatic compilation script.

```sh
npm run build:release
```

The Release version is compiled by default. If you need the Debug version, just run `npm run build:debug`.  
For Android, there is no need to manually call compilation. You can directly use Android Studio to open [Android](./examples/Android).

## License

[LGPL](./LICENSE) Copyright (c) 2024 mycrl.
