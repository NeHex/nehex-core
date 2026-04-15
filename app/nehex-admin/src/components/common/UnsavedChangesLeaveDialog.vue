<template>
  <v-dialog :model-value="modelValue" max-width="520" persistent @update:model-value="onDialogModelUpdate">
    <v-card class="unsaved-dialog" rounded="xl">
      <v-card-title class="unsaved-dialog-title">未保存更改</v-card-title>
      <v-card-text class="unsaved-dialog-text">
        当前页面有未保存的编辑内容。离开后，本次修改将丢失。
      </v-card-text>
      <v-card-actions class="unsaved-dialog-actions">
        <v-spacer />
        <v-btn variant="text" @click="handleCancel">
          继续编辑
        </v-btn>
        <v-btn color="warning" variant="flat" @click="handleConfirm">
          放弃并离开
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<script lang="ts" setup>
const props = defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
  (event: 'confirm'): void
  (event: 'cancel'): void
}>()

function onDialogModelUpdate(value: boolean): void {
  emit('update:modelValue', value)
  if (!value && props.modelValue) {
    emit('cancel')
  }
}

function handleConfirm(): void {
  emit('confirm')
  emit('update:modelValue', false)
}

function handleCancel(): void {
  emit('cancel')
  emit('update:modelValue', false)
}
</script>

<style scoped>
.unsaved-dialog {
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: linear-gradient(180deg, rgba(25, 32, 44, 0.98), rgba(18, 24, 34, 0.98));
  color: #edf2ff;
}

.unsaved-dialog-title {
  font-weight: 700;
  letter-spacing: 0.2px;
}

.unsaved-dialog-text {
  color: #c8d6f5;
  line-height: 1.7;
}

.unsaved-dialog-actions {
  padding: 8px 16px 16px;
}
</style>
