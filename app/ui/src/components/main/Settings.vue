<template>
    <div id="Settings">
        <div id="content">
            <!-- system -->
            <div class="module">
                <h1>{{ I18n.System.toLocaleUpperCase() }}</h1>

                <!-- Language -->
                <div class="item">
                    <p>{{ I18n.Language }}:</p>
                    <select :value="Language" @change="changeLanguage">
                        <option value="chinase">简体中文</option>
                        <option value="english">English</option>
                    </select>
                </div>
            </div>

            <!-- Network -->
            <div class="module">
                <h1>{{ I18n.Network.toLocaleUpperCase() }}</h1>

                <!-- 
                Interface

                Bound NIC interfaces, 0.0.0.0 means all NICs are bound. 
                 -->
                <div class="item">
                    <p>{{ I18n.NetworkInterface }}:</p>
                    <sub>{{ I18n.NetworkInterfaceTips }}</sub>
                    <input type="text" value="0.0.0.0" />
                </div>

                <!-- 
                MTU 

                In computer networking, the maximum transmission unit (MTU) is 
                the size of the largest protocol data unit (PDU) that can be 
                communicated in a single network layer transaction.
                -->
                <div class="item">
                    <p>{{ I18n.NetworkMtu }}:</p>
                    <sub>{{ I18n.NetworkMtuTips }}</sub>
                    <input type="number" value="1500" />
                </div>
            </div>

            <!-- Codec -->
            <div class="module">
                <h1>{{ I18n.Codec.toLocaleUpperCase() }}</h1>

                <!-- 
                Decoder
                
                Video decoder, H264 is a software decoder with the best compatibility.
                -->
                <div class="item">
                    <p>{{ I18n.CodecDecoder }}:</p>
                    <sub>{{ I18n.CodecDecoderTips }}</sub>
                    <select>
                        <option value="h264">H264</option>
                        <option value="d3d11va">D3D11VA - Windows</option>
                        <option value="qsv">Intel QSV - Windows</option>
                        <option value="videotoolbox">VideoToolbox - Apple</option>
                    </select>
                </div>

                <!-- 
                Encoder 

                Video encoder, X264 is a software encoder with the best compatibility.
                -->
                <div class="item">
                    <p>{{ I18n.CodecEncoder }}:</p>
                    <sub>{{ I18n.CodecEncoderTips }}</sub>
                    <select>
                        <option value="x264">X264</option>
                        <option value="qsv">Intel QSV - Windows</option>
                        <option value="videotoolbox">VideoToolbox - Apple</option>
                    </select>
                </div>
            </div>

            <!-- Video -->
            <div class="module">
                <h1>{{ I18n.Video.toLocaleUpperCase() }}</h1>

                <!-- 
                Size 

                The width and height of the video on the sender side.
                -->
                <div class="item">
                    <p>{{ I18n.VideoSize }}:</p>
                    <sub>{{ I18n.VideoSizeTips }}</sub>
                    <div>
                        <input type="number" value="1280" /> -
                        <input type="number" value="720" />
                    </div>
                </div>

                <!-- 
                FrameRate 

                The refresh rate of the video is usually 24 / 30 / 60.
                -->
                <div class="item">
                    <p>{{ I18n.VideoFrameRate }}:</p>
                    <sub>{{ I18n.VideoFrameRateTips }}</sub>
                    <input type="number" value="24" />
                </div>

                <!-- 
                BitRate 

                The bit rate of the video stream, in bit/s.
                -->
                <div class="item">
                    <p>{{ I18n.BitRate }}:</p>
                    <sub>{{ I18n.VideoBitRateTips }}</sub>
                    <input type="number" value="10000000" />
                </div>

                <!-- 
                KeyFrameInterval 

                It is recommended that the key frame interval be consistent with 
                the video frame rate, which helps reduce the size of the video stream.
                -->
                <div class="item">
                    <p>{{ I18n.VideoKeyFrameInterval }}:</p>
                    <sub>{{ I18n.VideoKeyFrameIntervalTips }}</sub>
                    <input type="number" value="24" />
                </div>
            </div>

            <!-- Audio -->
            <div class="module">
                <h1>{{ I18n.Audio.toLocaleUpperCase() }}</h1>

                <!-- 
                SampleRate 

                The audio sampling rate is recommended to be 48Khz.
                -->
                <div class="item">
                    <p>{{ I18n.AudioSampleRate }}:</p>
                    <sub>{{ I18n.AudioSampleRateTips }}</sub>
                    <input type="number" value="48000" />
                </div>

                <!-- 
                BitRate 

                The bit rate of the audio stream, in bit/s.
                -->
                <div class="item">
                    <p>{{ I18n.BitRate }}:</p>
                    <sub>{{ I18n.AudioBitRateTips }}</sub>
                    <input type="number" value="64000" />
                </div>
            </div>
        </div>

        <!-- apply button -->
        <button id="apply" class="click">
            {{ I18n.Apply }}
        </button>
    </div>
</template>

<script setup>
import { I18n, Language, setLanguage } from "@/i18n";

function changeLanguage({ target }) {
    setLanguage(target.value);
}
</script>

<style scoped>
#Settings {
    position: absolute;
    width: 100%;
    height: 100%;
}

#content {
    position: absolute;
    width: 975px;
    top: 100px;
    bottom: 100px;
    padding-left: 25px;
    overflow-y: scroll;
}

#content .module {
    margin-bottom: 30px;
}

#content .module h1 {
    font-size: 20px;
    color: #999;
    margin-bottom: 20px;
    color: #829bff;
    font-weight: 300;
}

#content .module .item {
    margin-top: 7px;
}

#content .module .item > * {
    display: block;
}

#content .module .item sub {
    color: #999;
}

#content .module .item input,
#content .module .item select {
    width: 214px;
    line-height: 30px;
    height: 30px;
    border-radius: 5px;
    background-color: #eee;
    border: 1px solid #ddd;
    text-indent: 10px;
    color: #000;
    margin: 2px 0;
}

#apply {
    height: 35px;
    width: 100px;
    background-color: #3951af;
    border-radius: 35px;
    border: 0;
    color: #fff;
    position: absolute;
    left: 20px;
    bottom: 20px;
}
</style>
