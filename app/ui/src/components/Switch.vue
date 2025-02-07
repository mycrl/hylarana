<template>
    <div id="Switch" class="click" @click="click">
        <div class="round" :id="state ? 'selected' : ''"></div>
    </div>
</template>

<script setup lang="ts">
import { ref } from "vue";

const props = defineProps<{
    value: boolean;
}>();

const emit = defineEmits(["change"]);

const state = ref(props.value || false);

function click() {
    state.value = !state.value;

    emit("change", state.value);
}
</script>

<style scoped>
#Switch {
    background-color: #ddd;
    height: 19px;
    width: 36px;
    border-radius: 19px;
    position: relative;
    box-shadow: inset 0 1px 1px #999;
}

.round {
    border-radius: 19px;
    height: 19px;
    width: 19px;
    background-color: #555;
    position: absolute;
    top: 0;
    left: 0;
}

#selected {
    background-color: #3951af;
    right: 0;
    left: auto;
}
</style>
