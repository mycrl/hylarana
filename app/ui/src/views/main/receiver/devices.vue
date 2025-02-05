<template>
    <div id="Devices">
        <div class="header">
            <p>{{ Text.Devices }}</p>
            <span>All devices that are casting the screen.</span>
        </div>
        <div class="items">
            <div class="device" v-for="it of devices">
                <div class="logo">
                    <FontAwesomeIcon class="icon" v-if="it.type == 'windows'" :icon="faWindows" />
                    <FontAwesomeIcon class="icon" v-if="it.type == 'android'" :icon="faAndroid" />
                    <FontAwesomeIcon class="icon" v-if="it.type == 'apple'" :icon="faApple" />
                </div>
                <div class="info">
                    <p>{{ it.name }}</p>
                    <span>{{ it.addr }}</span>
                </div>
                <div class="description">
                    <div class="item">
                        <p>{{ Text.Video }} -</p>
                        <span
                            >{{ Text.Codec }}: H264 / {{ Text.VideoSize }}: 1280x720 /
                            {{ Text.VideoFrameRate }}: 30 / {{ Text.BitRate }}: 10000000 /
                            {{ Text.VideoFormat }}: NV12</span
                        >
                    </div>
                    <div class="item">
                        <p>{{ Text.Audio }} -</p>
                        <span
                            >{{ Text.Codec }}: OPUS / {{ Text.AudioChannel }}:
                            {{ Text.AudioStereo }} / {{ Text.AudioSampleRate }}: 48000 /
                            {{ Text.AudioSampleBit }}: 16 / {{ Text.BitRate }}: 64000</span
                        >
                    </div>
                </div>
                <div class="accept click">
                    <span>Accapt</span>
                </div>
            </div>
            <span id="tips">
                {{ Text.DevicesSearchHelp }}
            </span>
        </div>
    </div>
</template>

<script setup>
import { ref } from "vue";

import Text from "@/text";
import { FontAwesomeIcon } from "@fortawesome/vue-fontawesome";
import { faWindows, faApple, faAndroid } from "@fortawesome/free-brands-svg-icons";

const devices = ref([
    {
        type: "windows",
        name: "Apple Lepidodendraceae",
        addr: "192.168.0.100",
    },
]);
</script>

<style scoped>
.header {
    border-bottom: 1px solid #e3e9ff;
}

.header p {
    color: #829bff;
    text-transform: uppercase;
}

.header span {
    color: #999;
    display: block;
    margin-bottom: 2px;
}

.items {
    width: 958px;
    height: 455px;
}

.items #tips {
    position: absolute;
    left: 0;
    bottom: 5px;
    width: 100%;
    text-align: center;
    color: #999;
    z-index: 1;
    overflow-y: auto;
    overflow-x: hidden;
}

.items .device {
    border: 1px solid #ddd;
    background-color: #eee;
    height: 50px;
    width: 100%;
    display: flex;
    margin-top: 10px;
    border-radius: 5px;
}

.items .device .logo {
    height: 52px;
    width: 52px;
    border-radius: 5px 0 0 5px;
    background-color: #829bff;
    color: #fff;
    text-align: center;
    line-height: 50px;
    position: relative;
    top: -1px;
    left: -1px;
}

.items .device .logo .icon {
    font-size: 20px;
    position: relative;
    top: 3px;
}

.items .device .info {
    padding: 7px 15px;
    border-right: 1px solid #ddd;
    width: 160px;
}

.items .device .info p {
    margin-top: 1px;
    margin-bottom: 4px;
}

.items .device .info span {
    color: #555;
}

.items .device .description {
    width: 600px;
    padding-left: 15px;
    padding-right: 0;
    padding-top: 9px;
}

.items .device .description .item p {
    display: inline;
    color: #829bff;
}

.items .device .accept {
    position: absolute;
    border-left: 1px solid #ddd;
    right: 0;
    height: 51px;
    width: 70px;
    text-align: center;
    line-height: 49px;
}
</style>
