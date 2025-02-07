<template>
    <div id="Header">
        <div id="navigation">
            <FontAwesomeIcon
                class="icon click transition"
                :icon="props.type == 'settings' ? faAngleLeft : faGear"
                @click="() => change(props.type == 'settings' ? 'sender' : 'settings')"
            />
            <span>
                {{ props.type != "settings" ? Locales.Settings : Locales.BackToHome }}
            </span>
        </div>
        <div id="switch" v-if="props.type != 'settings'">
            <div
                class="item click transition left"
                :id="props.type == 'sender' ? 'selected' : ''"
                @click="() => change('sender')"
            >
                <p>{{ Locales.Sender }}</p>
            </div>
            <div
                class="item click transition right"
                :id="props.type == 'receiver' ? 'selected' : ''"
                @click="() => change('receiver')"
            >
                <p>{{ Locales.Receiver }}</p>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import Locales from "@/locales";

import { FontAwesomeIcon } from "@fortawesome/vue-fontawesome";
import { faGear, faAngleLeft } from "@fortawesome/free-solid-svg-icons";

const props = defineProps(["type"]);
const emit = defineEmits(["change"]);

function change(kind: string) {
    emit("change", kind);
}
</script>

<style scoped>
#Header {
    user-select: none;
    position: fixed;
    top: 20px;
    left: 20px;
    z-index: 10;
    display: flex;
}

#switch {
    background-color: #3951af;
    width: 212px;
    height: 44px;
    border-radius: 44px;
    position: relative;
    box-shadow: 0 4px 4px rgba(0, 0, 0, 0.25);
    margin-left: 300px;
}

#navigation .icon {
    color: #3951af;
    font-size: 17px;
}

#navigation span {
    color: #555;
    margin-left: 5px;
    position: relative;
    top: -2px;
}

#switch .item {
    width: 106px;
    height: 44px;
    border-radius: 44px;
}

#switch .item p {
    text-align: center;
    line-height: 44px;
}

#switch .left {
    position: absolute;
    left: 0;
}

#switch .right {
    position: absolute;
    right: 0;
}

#switch #selected {
    background-color: #829bff;
}

#switch #selected p {
    color: #fff;
}
</style>
