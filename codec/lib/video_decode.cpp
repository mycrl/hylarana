//
//  video_encoder.cpp
//  codec
//
//  Created by Mr.Panda on 2024/2/14.
//

#include <string>

#include "./codec.h"

extern "C"
{
#include <libavutil/opt.h>
}

struct VideoDecoder* codec_create_video_decoder(const char* codec_name)
{
    std::string decoder = std::string(codec_name);
    struct VideoDecoder* codec = new VideoDecoder{};
    codec->output_frame = new VideoFrame{};

    codec->codec = avcodec_find_decoder_by_name(codec_name);
    if (codec->codec == nullptr)
    {
        codec_release_video_decoder(codec);
        return nullptr;
    }

    codec->context = avcodec_alloc_context3(codec->codec);
    if (codec->context == nullptr)
    {
        codec_release_video_decoder(codec);
        return nullptr;
    }

    codec->context->delay = 0;
    codec->context->max_samples = 1;
    codec->context->has_b_frames = 0;
    codec->context->thread_count = 1;
    codec->context->skip_alpha = true;
    codec->context->pix_fmt = AV_PIX_FMT_NV12;
    codec->context->flags |= AV_CODEC_FLAG_LOW_DELAY;

#ifdef WIN32
    if (decoder == "h264")
    {
        if (av_hwdevice_ctx_create(&codec->hw_device_ctx,
                                   AV_HWDEVICE_TYPE_DXVA2,
                                   NULL,
                                   NULL,
                                   0) < 0)
        {
            codec_release_video_decoder(codec);
            return nullptr;
        }

        codec->context->hw_device_ctx = av_buffer_ref(codec->hw_device_ctx);
    }
#endif

    if (decoder == "h264_qsv")
    {
        av_opt_set_int(codec->context->priv_data, "async_depth", 1, 0);
    }

    if (avcodec_open2(codec->context, codec->codec, nullptr) != 0)
    {
        codec_release_video_decoder(codec);
        return nullptr;
    }

    if (avcodec_is_open(codec->context) == 0)
    {
        codec_release_video_decoder(codec);
        return nullptr;
    }

    codec->parser = av_parser_init(codec->codec->id);
    if (!codec->parser)
    {
        codec_release_video_decoder(codec);
        return nullptr;
    }

    codec->packet = av_packet_alloc();
    if (codec->packet == nullptr)
    {
        codec_release_video_decoder(codec);
        return nullptr;
    }

    codec->frame = av_frame_alloc();
    if (codec->frame == nullptr)
    {
        codec_release_video_decoder(codec);
        return nullptr;
    }

    codec->sw_frame = av_frame_alloc();
    if (codec->sw_frame == nullptr)
    {
        codec_release_video_decoder(codec);
        return nullptr;
    }

    return codec;
}

void codec_release_video_decoder(struct VideoDecoder* codec)
{
    if (codec->context != nullptr)
    {
        avcodec_free_context(&codec->context);
    }

    if (codec->parser != nullptr)
    {
        av_parser_close(codec->parser);
    }

    if (codec->packet != nullptr)
    {
        av_packet_free(&codec->packet);
    }

    if (codec->frame != nullptr)
    {
        av_frame_free(&codec->frame);
    }

    if (codec->sw_frame != nullptr)
    {
        av_frame_free(&codec->sw_frame);
    }

    if (codec->hw_device_ctx != nullptr)
    {
        av_buffer_unref(&codec->hw_device_ctx);
    }

    delete codec->output_frame;
    delete codec;
}

bool codec_video_decoder_send_packet(struct VideoDecoder* codec,
                                     uint8_t* buf,
                                     size_t size)
{
    if (buf == nullptr)
    {
        return true;
    }

    while (size)
    {
        int len = av_parser_parse2(codec->parser,
                                   codec->context,
                                   &codec->packet->data,
                                   &codec->packet->size,
                                   buf,
                                   size,
                                   AV_NOPTS_VALUE,
                                   AV_NOPTS_VALUE,
                                   0);
        buf += len;
        size -= len;

        if (codec->packet->size)
        {
            if (avcodec_send_packet(codec->context, codec->packet) != 0)
            {
                return false;
            }
        }
    }

    return true;
}

struct VideoFrame* codec_video_decoder_read_frame(struct VideoDecoder* codec)
{
    if (avcodec_receive_frame(codec->context, codec->frame) != 0)
    {
        return nullptr;
    }

    codec->output_frame->rect.width = codec->frame->width;
    codec->output_frame->rect.height = codec->frame->height;

    if (codec->frame->format == AV_PIX_FMT_DXVA2_VLD)
    {
        if (av_hwframe_transfer_data(codec->sw_frame, codec->frame, 0) < 0)
        {
            return nullptr;
        }
    }

    auto frame = codec->frame->format == AV_PIX_FMT_DXVA2_VLD ? 
        codec->sw_frame : 
        codec->frame;

    for (int i = 0; i < 2; i++)
    {
        codec->output_frame->linesize[i] = frame->linesize[i];
        codec->output_frame->data[i] = frame->data[i];
    }

    return codec->output_frame;
}
