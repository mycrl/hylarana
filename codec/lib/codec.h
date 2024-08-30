//
//  codec.h
//  codec
//
//  Created by Panda on 2024/2/14.
//

#ifndef CODEC_H
#define CODEC_H
#pragma once

#ifndef EXPORT
#ifdef WIN32
#define EXPORT __declspec(dllexport)
#else
#define EXPORT
#endif
#endif

#include <string>
#include <vector>
#include <optional>

#ifdef WIN32
#include <d3d11_4.h>
#endif // WIN32

extern "C"
{
#include <frame.h>
#include <libavutil/hwcontext_qsv.h>
#include <libavutil/hwcontext.h>
#include <libavcodec/avcodec.h>
#include <libavutil/frame.h>

#ifdef WIN32
#include <libavutil/hwcontext_d3d11va.h>
#endif // WIN32
}

struct CodecContext
{
	const AVCodec* codec;
	AVCodecContext* context;
};

struct Packet
{
	uint8_t* buffer;
	size_t len;
	int flags;
    uint64_t timestamp;
};

struct VideoEncoderSettings
{
#ifdef WIN32
	ID3D11Device* d3d11_device;
	ID3D11DeviceContext* d3d11_device_context;
#endif // WIN32
	const char* codec;
	uint8_t frame_rate;
	uint32_t width;
	uint32_t height;
	uint64_t bit_rate;
	uint32_t key_frame_interval;
};

struct VideoEncoder
{
	bool initialized;
	AVCodecContext* context;
	AVPacket* packet;
	AVFrame* frame;
	Packet* output_packet;
};

struct VideoDecoderSettings
{
#ifdef WIN32
	ID3D11Device* d3d11_device;
	ID3D11DeviceContext* d3d11_device_context;
#endif // WIN32
	const char* codec;
};

struct VideoDecoder
{
	AVCodecContext* context;
	AVCodecParserContext* parser;
	AVPacket* packet;
	AVFrame* frame;
	VideoFrame* output_frame;
};

struct AudioEncoderSettings
{
	const char* codec;
	uint64_t bit_rate;
	uint64_t sample_rate;
};

struct AudioEncoder
{
	AVCodecContext* context;
	AVPacket* packet;
	AVFrame* frame;
	Packet* output_packet;
	uint64_t pts;
};

struct AudioDecoderSettings
{
	const char* codec;
};

struct AudioDecoder
{
	AVCodecContext* context;
	AVCodecParserContext* parser;
	AVPacket* packet;
	AVFrame* frame;
	AudioFrame* output_frame;
};

struct CodecDesc
{
	const char* name;
	AVHWDeviceType type;
};

enum CodecKind
{
	Encoder,
	Decoder,
};

typedef void (*Logger)(int level, char* message);

extern "C"
{
	EXPORT void codec_set_logger(Logger logger);
	EXPORT void codec_remove_logger();
	EXPORT const char* codec_find_video_encoder();
	EXPORT const char* codec_find_video_decoder();
	EXPORT VideoEncoder* codec_create_video_encoder(VideoEncoderSettings* settings);
    EXPORT bool codec_video_encoder_copy_frame(VideoEncoder* codec, VideoFrame* frame);
	EXPORT bool codec_video_encoder_send_frame(VideoEncoder* codec);
	EXPORT Packet* codec_video_encoder_read_packet(VideoEncoder* codec);
	EXPORT void codec_unref_video_encoder_packet(VideoEncoder* codec);
	EXPORT void codec_release_video_encoder(VideoEncoder* codec);
	EXPORT VideoDecoder* codec_create_video_decoder(VideoDecoderSettings* settings);
	EXPORT void codec_release_video_decoder(VideoDecoder* codec);
	EXPORT bool codec_video_decoder_send_packet(VideoDecoder* codec, Packet packet);
	EXPORT VideoFrame* codec_video_decoder_read_frame(VideoDecoder* codec);
	EXPORT AudioEncoder* codec_create_audio_encoder(AudioEncoderSettings* settings);
    EXPORT bool codec_audio_encoder_copy_frame(AudioEncoder* codec, AudioFrame* frame);
	EXPORT bool codec_audio_encoder_send_frame(AudioEncoder* codec);
	EXPORT Packet* codec_audio_encoder_read_packet(AudioEncoder* codec);
	EXPORT void codec_unref_audio_encoder_packet(AudioEncoder* codec);
	EXPORT void codec_release_audio_encoder(AudioEncoder* codec);
	EXPORT AudioDecoder* codec_create_audio_decoder(AudioDecoderSettings* settings);
	EXPORT void codec_release_audio_decoder(AudioDecoder* codec);
	EXPORT bool codec_audio_decoder_send_packet(AudioDecoder* codec, Packet* packet);
	EXPORT AudioFrame* codec_audio_decoder_read_frame(AudioDecoder* codec);
}

#ifdef WIN32
std::optional<CodecContext> create_video_context(CodecKind kind, 
												 std::string& codec, 
												 int width,
												 int height,
												 ID3D11Device* d3d11_device, 
												 ID3D11DeviceContext* d3d11_device_context);
#else
std::optional<CodecContext> create_video_context(CodecKind kind, std::string& codec);
#endif // WIN32

AVFrame* create_video_frame(AVCodecContext* context);

#endif // CODEC_H
