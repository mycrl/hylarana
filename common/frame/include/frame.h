//
//  codec.h
//  codec
//
//  Created by Panda on 2024/2/14.
//

#ifndef FRAME_H
#define FRAME_H
#pragma once

#include <stddef.h>
#include <stdint.h>

typedef enum
{
    RGBA,
    NV12,
    I420,
} VideoFormat;

typedef struct
{
    VideoFormat format;
    bool hardware;
    uint32_t width;
    uint32_t height;
    void* data[2];
    size_t linesize[2];
} VideoFrame;

typedef struct
{
    int sample_rate;
    uint32_t frames;
    int16_t* data;
} AudioFrame;

#endif /* FRAME_H */