<template>
    <div id="Header">
        <div id="icon">
            <img
                style="width: 20px"
                v-show="props.type != 'settings'"
                class="click transition"
                src="@/assets/icons/settings.svg"
                @click="() => change('settings')"
            />
            <img
                v-show="props.type == 'settings'"
                class="click transition"
                src="@/assets/icons/arrow_left.svg"
                @click="() => change('sender')"
            />
            <span>{{ props.type != "settings" ? I18n.Settings : I18n.BackToHome }}</span>
        </div>
        <div id="switch" v-show="props.type != 'settings'">
            <div
                class="item click transition left"
                :id="props.type == 'sender' ? 'selected' : null"
                @click="() => change('sender')"
            >
                <p>{{ I18n.Sender }}</p>
            </div>
            <div
                class="item click transition right"
                :id="props.type == 'receiver' ? 'selected' : null"
                @click="() => change('receiver')"
            >
                <p>{{ I18n.Receiver }}</p>
            </div>
        </div>
    </div>
</template>

<script setup>
import { ref } from "vue";
import { I18n } from "@/i18n";

const props = defineProps(["type"]);
const emit = defineEmits(["change"]);

function change(kind) {
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

#icon span {
    color: #555;
    position: relative;
    top: -7px;
    margin-left: 5px;
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
