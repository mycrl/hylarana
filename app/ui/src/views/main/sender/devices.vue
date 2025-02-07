<template>
    <div id="Devices">
        <div class="header">
            <p>{{ Locales.Devices }}</p>
            <span>{{ Locales.DevicesHelp }}</span>
        </div>
        <div class="items">
            <div class="device click" v-for="it of props.devices" v-if="it && it.description">
                <div class="logo">
                    <FontAwesomeIcon class="icon" v-if="it.kind == 'Windows'" :icon="faWindows" />
                    <FontAwesomeIcon class="icon" v-if="it.kind == 'Android'" :icon="faAndroid" />
                    <FontAwesomeIcon class="icon" v-if="it.kind == 'Apple'" :icon="faApple" />
                </div>
                <div class="info">
                    <p>{{ it.name }}</p>
                    <span>{{ it.addrs[0] }}</span>
                </div>
            </div>
            <span id="tips">
                {{ Locales.DevicesSearchHelp }}
            </span>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { DeviceInfo } from "@/interfaces";
import Locales from "@/locales";
import { FontAwesomeIcon } from "@fortawesome/vue-fontawesome";
import { faWindows, faApple, faAndroid } from "@fortawesome/free-brands-svg-icons";

const props = defineProps<{
    devices: Array<DeviceInfo>;
}>();
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
    width: 965px;
    height: 320px;
    position: relative;
    display: table;
    left: -5px;
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
    border-radius: 5px;
    height: 50px;
    width: 234px;
    display: flex;
    margin-left: 5px;
    margin-top: 5px;
    float: left;
}

.items .device .logo {
    border-radius: 5px;
    height: 52px;
    width: 52px;
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
    padding: 7px;
}

.items .device .info p {
    margin-top: 1px;
    margin-bottom: 4px;
}

.items .device .info span {
    color: #555;
}
</style>
