//
//  codec.h
//  codec
//
//  Created by Panda on 2024/2/14.
//

#ifndef codec_h
#define codec_h
#pragma once

#ifdef WIN32
#define EXPORT __declspec(dllexport)
#else
#define EXPORT
#endif

#include <string>

#include <frame.h>

extern "C"
{
#include <libavcodec/avcodec.h>
#include <libavutil/frame.h>
}

struct EncodePacket
{
	uint8_t* buffer;
	size_t len;
	int flags;
};

struct VideoEncoderSettings
{
	const char* codec_name;
	uint8_t max_b_frames;
	uint8_t frame_rate;
	uint32_t width;
	uint32_t height;
	uint64_t bit_rate;
	uint32_t key_frame_interval;
};

struct VideoEncoder
{
	std::string codec_name;
	const AVCodec* codec;
	AVCodecContext* context;
	AVPacket* packet;
	AVFrame* frame;
	uint64_t frame_num;
	struct EncodePacket* output_packet;
};

struct VideoDecoder
{
	const AVCodec* codec;
	AVCodecContext* context;
	AVCodecParserContext* parser;
	AVPacket* packet;
	AVFrame* frame;
	struct VideoFrame* output_frame;
};

struct AudioEncoderSettings
{
    const char* codec_name;
    uint64_t bit_rate;
    uint64_t sample_rate;
};

struct AudioEncoder
{
    std::string codec_name;
    const AVCodec* codec;
    AVCodecContext* context;
	AVPacket* packet;
	AVFrame* frame;
    uint64_t frame_num;
    struct EncodePacket* output_packet;
};

extern "C"
{
	EXPORT struct VideoEncoder* codec_create_video_encoder(struct VideoEncoderSettings* settings);
	EXPORT bool codec_video_encoder_send_frame(struct VideoEncoder* codec, struct VideoFrame* frame);
	EXPORT struct EncodePacket* codec_video_encoder_read_packet(struct VideoEncoder* codec);
	EXPORT void codec_unref_video_encoder_packet(struct VideoEncoder* codec);
	EXPORT void codec_release_video_encoder(struct VideoEncoder* codec);
	EXPORT struct VideoDecoder* codec_create_video_decoder(const char* codec_name);
	EXPORT void codec_release_video_decoder(struct VideoDecoder* decoder);
	EXPORT bool codec_video_decoder_send_packet(struct VideoDecoder* decoder, uint8_t* buf, size_t size);
	EXPORT struct VideoFrame* codec_video_decoder_read_frame(struct VideoDecoder* decoder);
    EXPORT struct AudioEncoder* codec_create_audio_encoder(struct AudioEncoderSettings* settings);
    EXPORT bool codec_audio_encoder_send_frame(struct AudioEncoder* codec, struct AudioFrame* frame);
    EXPORT struct EncodePacket* codec_audio_encoder_read_packet(struct AudioEncoder* codec);
    EXPORT void codec_unref_audio_encoder_packet(struct AudioEncoder* codec);
    EXPORT void codec_release_audio_encoder(struct AudioEncoder* codec);
}

#endif /* codec_h */
