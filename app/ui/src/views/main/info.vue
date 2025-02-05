<template>
    <div id="Info">
        <span>{{ Text.Network }}:</span>
        <p>{{ Text.Direct }}-{{ Settings.network.interface }}</p>
        <sub>/</sub>
        <span>{{ Text.Video }}:</span>
        <p>
            {{ Settings.video.size.width }}x{{ Settings.video.size.height }}/{{
                Settings.video.frameRate
            }}/{{ Settings.video.bitRate }}
        </p>
        <sub>/</sub>
        <span>{{ Text.Audio }}:</span>
        <p>{{ Text.AudioStereo }}/{{ Settings.audio.sampleRate }}/{{ Settings.audio.bitRate }}</p>

        <div class="timer">00:00:00</div>
    </div>
</template>

<script setup>
import Text from "@/text";
import Settings, { VideoEncoders } from "@/settings";
</script>

<style scoped>
#Info {
    position: absolute;
    bottom: 0;
    left: 20px;
    right: 20px;
    border-top: 1px solid #e3e9ff;
    display: flex;
    line-height: 30px;
}

#Info span {
    color: #829bff;
}

#Info p {
    color: #999999;
}

#Info sub {
    color: #555;
    margin: 0 5px;
}

#Info .timer {
    position: absolute;
    right: 0;
}
</style>
