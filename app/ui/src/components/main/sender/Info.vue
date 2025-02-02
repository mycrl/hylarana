<template>
    <div id="Info">
        <div class="item">
            <p>{{ Text.Video }}</p>
            <span>{{ Text.Type }}: H264</span>
            <span
                >{{ Text.CodecEncoder }}:
                {{ VideoEncoders[Settings.codec.encoder].split("-")[0].trim() }}</span
            >
            <span
                >{{ Text.VideoSize }}: {{ Settings.video.size.width }} x
                {{ Settings.video.size.height }}</span
            >
            <span>{{ Text.VideoFrameRate }}: {{ Settings.video.frameRate }}</span>
            <span>{{ Text.BitRate }}: {{ Settings.video.bitRate }}</span>
            <span>{{ Text.VideoKeyFrameInterval }}: {{ Settings.video.keyFrameInterval }}</span>
        </div>
        <div class="item">
            <p>{{ Text.Audio }}</p>
            <span>{{ Text.Type }}: OPUS</span>
            <span>{{ Text.AudioChannel }}: {{ Text.AudioStereo }}</span>
            <span>{{ Text.AudioSampleBit }}: 16</span>
            <span>{{ Text.AudioSampleRate }}: {{ Settings.audio.sampleRate }}</span>
            <span>{{ Text.BitRate }}: {{ Settings.audio.bitRate }}</span>
        </div>
        <div class="item">
            <p>{{ Text.Network }}</p>
            <span>{{ Text.NetworkMtu }}: {{ Settings.network.mtu }}</span>
            <span>{{ Text.NetworkInterface }}: {{ Settings.network.interface }}</span>
            <span>{{ Text.NetworkMulticast }}: {{ Settings.network.multicast }}</span>
            <span>{{ Text.NetworkServer }}: {{ Settings.network.server }}</span>
        </div>
    </div>
</template>

<script setup>
import Text from "@/text";
import Settings, { VideoEncoders } from "@/settings";
</script>

<style scoped>
#Info {
    width: 50%;
    height: 100%;
    border-right: 1px solid #e3e9ff;
    display: flex;
}

.item {
    margin-top: 10px;
    flex: 1;
}

.item:nth-child(3) {
    flex: 1.5;
}

.item p {
    color: #829bff;
    margin-bottom: 10px;
    text-transform: uppercase;
}

.item span {
    display: block;
    color: #555;
}
</style>
