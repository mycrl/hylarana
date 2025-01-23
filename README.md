<!--lint disable no-literal-urls-->
<div align="center">
   <h1>Hylarana</h1>
</div>
<br/>
<div align="center">
  <strong>A cross-platform screen casting library implemented by rust.</strong>
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
  <a href="./examples/android">android</a>
</div>
<div align="center">
  <span>watch the demo on youtube:</span>
  <a href="https://www.youtube.com/shorts/npD_VgEKZmM">link</a>
</div>
<br/>
<br/>

---

Unlike implementations such as Miracast, AirPlay, etc. that rely on hardware support (WIFI Direct), this library works on most common hardware.

The project is cross-platform, but the priority platforms supported are Windows and Android, Unlike a solution like DLNA, this project is more akin to airplay, so low latency is the main goal, currently the latency is controlled at around 80-250ms (with some variations on different platforms with different codecs), and maintains a highly easy to use API and very few external dependencies.

Unlike traditional screen casting implementations, this project can work in forwarding mode, in which it can support casting to hundreds or thousands of devices at the same time, which can be useful in some specific scenarios (e.g., all advertising screens in a building).

## How was this achieved?

First of all screen capture, this part of the implementation of each platform independently separate, currently windows and android capture the highest efficiency, because the use of hardware-accelerated textures, android uses the virtual display, windows uses the WGC, and linux only uses the x11grab, so the efficiency of linux is poorer.

For audio and video codecs, H264 is used for video and Opus is used for audio, again, hardware accelerated codecs are available for both windows and android. Currently, the hardware codecs on windows are adapted to Qsv and D3D11VA, while android is adapted to Qualcomm, Kirin, and RK series of socs.

Both SRT and UDP multicast schemes are used for the transport layer of the data. The audio and video data transmitted by the transport layer are bare streams and do not contain similar encapsulations such as FLV. For SRT, many parameters have been adjusted in detail to suit the LAN environment, so that when using the SRT transport layer, the delay can be controlled at about 20-40 ms. The UDP multicast scheme has only a receive buffer and no transmit buffer, and the fixed maximum delay of UDP multicast is 40 ms, which is used to sort and wait for packets in the buffer.

The graphics interface also uses both Direct3D11 and WebGPU solutions, WebGPU is a cross-platform graphics interface wrapper library, but on some older devices on windows, WebGPU does not work because WebGPU requires a minimum of Direct3D12 support, so Direct3D11 is provided on windows programme. Similarly, the graphics implementations for windows and android are fully hardware accelerated, in general, a video frame is captured, encoded, decoded and displayed within the GPU, and scaling and format conversion for video frames on windows is also fully hardware accelerated.

## Build Instructions

#### Requirements

-   [Git](https://git-scm.com/downloads)
-   [Rust](https://www.rust-lang.org/tools/install): Rust stable toolchain.
-   C++20 or above compliant compiler. (G++/Clang/MSVC)
-   [CMake](https://cmake.org/download/): CMake 3.16 or above as a build system.
-   [Node.js](https://nodejs.org/en/download): Node.js 16 or above as a auto build script.
-   [Cargo NDK](https://github.com/willir/cargo-ndk-android-gradle): Cargo NDK is optional and required for Android Studio projects.

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
For android, there is no need to manually call compilation. You can directly use Android Studio to open [android](./examples/android).

## License

[LGPL](./LICENSE) Copyright (c) 2024 mycrl.
