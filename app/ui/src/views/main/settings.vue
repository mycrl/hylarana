<template>
    <div id="Settings">
        <div id="content">
            <!-- system -->
            <div class="module">
                <h1>{{ Locales.System }}</h1>

                <!-- Device Name -->
                <div class="item">
                    <p>{{ Locales.DeviceName }}:</p>
                    <input type="text" v-model="Settings.system.deviceName" :disabled="disabled" />
                </div>

                <!-- Language -->
                <div class="item">
                    <p>{{ Locales.Language }}:</p>
                    <select v-model="Settings.system.language" :disabled="disabled">
                        <option value="chinase">简体中文</option>
                        <option value="english">English</option>
                    </select>
                </div>
            </div>

            <!-- Network -->
            <div class="module">
                <h1>{{ Locales.Network }}</h1>

                <!-- 
                Interface

                Bound NIC interfaces, 0.0.0.0 means all NICs are bound. 
                 -->
                <div class="item">
                    <p>{{ Locales.NetworkInterface }}:</p>
                    <sub>{{ Locales.NetworkInterfaceHelp }}</sub>
                    <input type="text" v-model="Settings.network.interface" :disabled="disabled" />
                </div>

                <!-- 
                Multicast

                The IP address used for multicast, the default is 239.0.0.1.
                 -->
                <div class="item">
                    <p>{{ Locales.NetworkMulticast }}:</p>
                    <sub>{{ Locales.NetworkMulticastHelp }}</sub>
                    <input type="text" v-model="Settings.network.multicast" :disabled="disabled" />
                </div>

                <!-- 
                Server

                The address of the forwarding server, such as 192.168.1.100:8080.
                 -->
                <div class="item">
                    <p>{{ Locales.NetworkServer }}:</p>
                    <sub>{{ Locales.NetworkServerHelp }}</sub>
                    <input type="text" v-model="Settings.network.server" :disabled="disabled" />
                </div>

                <!-- 
                MTU 

                In computer networking, the maximum transmission unit (MTU) is 
                the size of the largest protocol data unit (PDU) that can be 
                communicated in a single network layer transaction.
                -->
                <div class="item">
                    <p>{{ Locales.NetworkMtu }}:</p>
                    <sub>{{ Locales.NetworkMtuHelp }}</sub>
                    <input
                        type="number"
                        v-model.number="Settings.network.mtu"
                        :disabled="disabled"
                    />
                </div>
            </div>

            <!-- Codec -->
            <div class="module">
                <h1>{{ Locales.Codec }}</h1>

                <!-- 
                Decoder
                
                Video decoder, H264 is a software decoder with the best compatibility.
                -->
                <div class="item">
                    <p>{{ Locales.CodecDecoder }}:</p>
                    <sub>{{ Locales.CodecDecoderHelp }}</sub>
                    <select v-model="Settings.codec.decoder" :disabled="disabled">
                        <option v-for="(v, k) in VideoDecoders" :value="k">{{ v }}</option>
                    </select>
                </div>

                <!-- 
                Encoder 

                Video encoder, X264 is a software encoder with the best compatibility.
                -->
                <div class="item">
                    <p>{{ Locales.CodecEncoder }}:</p>
                    <sub>{{ Locales.CodecEncoderHelp }}</sub>
                    <select v-model="Settings.codec.encoder" :disabled="disabled">
                        <option v-for="(v, k) in VideoEncoders" :value="k">{{ v }}</option>
                    </select>
                </div>
            </div>

            <!-- Video -->
            <div class="module">
                <h1>{{ Locales.Video }}</h1>

                <!-- 
                Size 

                The width and height of the video on the sender side.
                -->
                <div class="item">
                    <p>{{ Locales.VideoSize }}:</p>
                    <sub>{{ Locales.VideoSizeHelp }}</sub>
                    <div>
                        <input
                            type="number"
                            v-model.number="Settings.video.size.width"
                            :disabled="disabled"
                        />
                        -
                        <input
                            type="number"
                            v-model.number="Settings.video.size.height"
                            :disabled="disabled"
                        />
                    </div>
                </div>

                <!-- 
                FrameRate 

                The refresh rate of the video is usually 24 / 30 / 60.
                -->
                <div class="item">
                    <p>{{ Locales.VideoFrameRate }}:</p>
                    <sub>{{ Locales.VideoFrameRateHelp }}</sub>
                    <input
                        type="number"
                        v-model.number="Settings.video.frameRate"
                        :disabled="disabled"
                    />
                </div>

                <!-- 
                BitRate 

                The bit rate of the video stream, in bit/s.
                -->
                <div class="item">
                    <p>{{ Locales.BitRate }}:</p>
                    <sub>{{ Locales.VideoBitRateHelp }}</sub>
                    <input
                        type="number"
                        v-model.number="Settings.video.bitRate"
                        :disabled="disabled"
                    />
                </div>

                <!-- 
                KeyFrameInterval 

                It is recommended that the key frame interval be consistent with 
                the video frame rate, which helps reduce the size of the video stream.
                -->
                <div class="item">
                    <p>{{ Locales.VideoKeyFrameInterval }}:</p>
                    <sub>{{ Locales.VideoKeyFrameIntervalHelp }}</sub>
                    <input
                        type="number"
                        v-model.number="Settings.video.keyFrameInterval"
                        :disabled="disabled"
                    />
                </div>
            </div>

            <!-- Audio -->
            <div class="module">
                <h1>{{ Locales.Audio }}</h1>

                <!-- 
                SampleRate 

                The audio sampling rate is recommended to be 48Khz.
                -->
                <div class="item">
                    <p>{{ Locales.AudioSampleRate }}:</p>
                    <sub>{{ Locales.AudioSampleRateHelp }}</sub>
                    <input
                        type="number"
                        v-model.number="Settings.audio.sampleRate"
                        :disabled="disabled"
                    />
                </div>

                <!-- 
                BitRate 

                The bit rate of the audio stream, in bit/s.
                -->
                <div class="item">
                    <p>{{ Locales.BitRate }}:</p>
                    <sub>{{ Locales.AudioBitRateHelp }}</sub>
                    <input
                        type="number"
                        v-model.number="Settings.audio.bitRate"
                        :disabled="disabled"
                    />
                </div>
            </div>
        </div>

        <!-- apply button -->
        <button v-if="!disabled" id="apply" class="click" @click="submit">
            {{ Locales.Apply }}
        </button>
    </div>
</template>

<script setup lang="ts">
import { ref } from "vue";

import Locales from "@/locales";
import { MessageRouter } from "@/message";
import Settings, { update as updateSettings, VideoEncoders, VideoDecoders } from "@/settings";

const disabled = ref(false);

function submit() {
    MessageRouter.call("SetName", Settings.value.system.deviceName).then(() => {
        updateSettings();

        disabled.value = true;
    });
}

MessageRouter.call("GetName").then((name) => {
    Settings.value.system.deviceName = name;
});
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
    bottom: 75px;
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
    text-transform: uppercase;
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
