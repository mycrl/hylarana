#ifndef ARGS_H
#define ARGS_H
#pragma once

extern "C"
{
#include <hylarana.h>
}

#include <stdexcept>
#include <string>
#include <vector>
#include <tuple>

static struct
{
#ifdef WIN32
    HylaranaVideoEncoderType encoder = VIDEO_ENCODER_QSV;
    HylaranaVideoDecoderType decoder = VIDEO_DECODER_D3D11;
#elif MACOS
    VideoEncoderType encoder = VIDEO_ENCODER_VIDEOTOOLBOX;
    VideoDecoderType decoder = VIDEO_DECODER_VIDEOTOOLBOX;
#else
    VideoEncoderType encoder = VIDEO_ENCODER_X264;
    VideoDecoderType decoder = VIDEO_DECODER_H264;
#endif
    HylaranaStrategy strategy = STRATEGY_DIRECT;
    std::string address = "127.0.0.1:8080";
    int width = 1280;
    int height = 720;
    int fps = 30;
} OPTIONS = {};

HylaranaVideoEncoderType encoder_from_str(std::string value)
{
    if (value == "libx264")
    {
        return VIDEO_ENCODER_X264;
    }
    else if (value == "h264_qsv")
    {
        return VIDEO_ENCODER_QSV;
    }
    else if (value == "h264_nvenc")
    {
        return VIDEO_ENCODER_CUDA;
    }
    else if (value == "h264_videotoolbox")
    {
        return VIDEO_ENCODER_VIDEOTOOLBOX;
    }
    else
    {
        throw std::invalid_argument("encoder");
    }
}

HylaranaVideoDecoderType decoder_from_str(std::string value)
{
    if (value == "h264")
    {
        return VIDEO_DECODER_H264;
    }
    else if (value == "d3d11va")
    {
        return VIDEO_DECODER_D3D11;
    }
    else if (value == "h264_qsv")
    {
        return VIDEO_DECODER_QSV;
    }
    else if (value == "h264_cuvid")
    {
        return VIDEO_DECODER_CUDA;
    }
    else if (value == "h264_videotoolbox")
    {
        return VIDEO_DECODER_VIDEOTOOLBOX;
    }
    else
    {
        throw std::invalid_argument("decoder");
    }
}

HylaranaStrategy strategy_from_str(std::string value)
{
    if (value == "direct")
    {
        return STRATEGY_DIRECT;
    }
    else if (value == "relay")
    {
        return STRATEGY_RELAY;
    }
    else if (value == "multicast")
    {
        return STRATEGY_MULTICAST;
    }
    else
    {
        throw std::invalid_argument("strategy");
    }
}

std::vector<std::string> finds(std::string input, std::string delimiter)
{
    size_t iter = 0;
    std::vector<std::string> tokens;
    while (iter < input.size())
    {
        iter = input.find(delimiter);
        tokens.push_back(input.substr(0, iter));
        input.erase(0, iter + delimiter.length());
    }

    if (input.size() > 0)
    {
        tokens.push_back(input);
    }

    return tokens;
}

std::tuple<std::string, std::string> get_key_value(std::string input,
                                                   std::string delimiter)
{
    std::vector<std::string> tokens = finds(input, delimiter);
    if (tokens.size() < 2)
    {
        throw std::invalid_argument(input);
    }

    return { tokens[0], tokens[1] };
}

int parse_argv(std::string args)
{
    for (auto path : finds(args, " "))
    {
        const auto [key, value] = get_key_value(path, "=");
        if (key == "--address")
        {
            OPTIONS.address = value;
        }
        else if (key == "--strategy")
        {
            OPTIONS.strategy = strategy_from_str(value);
        }
        else if (key == "--fps")
        {
            OPTIONS.fps = std::stoi(value);
        }
        else if (key == "--width")
        {
            OPTIONS.width = std::stoi(value);
        }
        else if (key == "--height")
        {
            OPTIONS.height = std::stoi(value);
        }
        else if (key == "--encoder")
        {
            OPTIONS.encoder = encoder_from_str(value);
        }
        else if (key == "--decoder")
        {
            OPTIONS.decoder = decoder_from_str(value);
        }
        else if (key == "--help")
        {
            printf("\n");
            printf("--fps       default=30              - frame rate\n");
            printf("--width     default=1280            - video width\n");
            printf("--height    default=720             - video height\n");
            printf("--encoder   default=*               - libx264, h264_qsv, h264_nvenc, h264_videotoolbox\n");
            printf("--decoder   default=*               - h264, d3d11va, h264_qsv, h264_cuvid, h264_videotoolbox\n");
            printf("--address   default=127.0.0.1:8080  - hylarana service bind address\n");
            printf("--strategy  default=direct          - direct, relay, multicast\n");
            printf("\n");
            return -1;
        }
    }

    return 0;
}

class SocketAddr
{
public:
    SocketAddr(std::string address)
    {
        auto items = finds(std::string(address), ":");
        _ip = items[0];
        _port = items[1];
    }

    std::string GetIP()
    {
        return _ip;
    }

    uint16_t GetPort()
    {
        return (uint16_t)std::stoi(_port);
    }

    std::string ToString()
    {
        return _ip + ":" + _port;
    }

    void SetPort(uint16_t port)
    {
        char port_str[10];
        sprintf(port_str, "%d", port);
        _port = std::string(std::move(port_str));
    }

    void SetPort(std::string port)
    {
        _port = port;
    }

    void SetIP(std::string ip)
    {
        _ip = ip;
    }
private:
    std::string _ip;
    std::string _port;
};

#endif