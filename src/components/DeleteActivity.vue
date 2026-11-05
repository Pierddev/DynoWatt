<script setup lang="ts">
import { useTemplateRef } from "vue";
import { useI18n } from "vue-i18n";
import { deleteActivity } from "../activities/commands";
import { useDeletion } from "../activities/useDeletion";

const props = defineProps<{ activityId: number }>();
const emit = defineEmits<{ deleted: [] }>();
const { t } = useI18n();
const { deleting, error, remove, dismiss } = useDeletion(deleteActivity);
const dialog = useTemplateRef<HTMLDialogElement>("dialog");

function ask() {
  dismiss();
  dialog.value?.showModal();
}

// A deletion under way runs to its end: neither Cancel nor Escape closes the dialog then.
function cancel() {
  if (!deleting.value) dialog.value?.close();
}

function onEscape(event: Event) {
  if (deleting.value) event.preventDefault();
}

async function confirm() {
  if (await remove(props.activityId)) {
    dialog.value?.close();
    emit("deleted");
  } else if (dialog.value && !dialog.value.open) {
    // The browser may close the dialog on Escape all the same: the failure must still show.
    dialog.value.showModal();
  }
}

const BUTTON =
  "rounded-md px-3 py-1.5 text-sm font-medium focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-neutral-500 disabled:cursor-not-allowed disabled:opacity-50";
</script>

<template>
  <button
    type="button"
    class="rounded-md px-2 py-1 text-sm font-medium text-red-700 hover:bg-red-50 focus-visible:outline-2 focus-visible:outline-neutral-500 dark:text-red-400 dark:hover:bg-red-950/50"
    @click="ask"
  >
    {{ t("detail.delete") }}
  </button>
  <dialog
    ref="dialog"
    aria-labelledby="delete-activity-title"
    aria-describedby="delete-activity-body"
    class="m-auto w-full max-w-md rounded-lg border border-neutral-200 bg-white p-6 text-neutral-900 shadow-xl backdrop:bg-black/40 dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-100"
    @cancel="onEscape"
  >
    <h2 id="delete-activity-title" class="text-lg font-semibold">{{ t("deleteActivity.title") }}</h2>
    <p id="delete-activity-body" class="mt-2 text-sm text-neutral-600 dark:text-neutral-300">
      {{ t("deleteActivity.body") }}
    </p>
    <p v-if="error" role="alert" class="mt-3 text-sm text-red-700 dark:text-red-400">
      {{ t("deleteActivity.error", { reason: error }) }}
    </p>
    <div class="mt-6 flex justify-end gap-2">
      <!-- Focused first: Enter right after opening keeps the Activity. -->
      <button
        type="button"
        autofocus
        :class="BUTTON"
        class="border border-neutral-300 enabled:hover:bg-neutral-100 dark:border-neutral-600 dark:enabled:hover:bg-neutral-700"
        :disabled="deleting"
        @click="cancel"
      >
        {{ t("deleteActivity.cancel") }}
      </button>
      <button
        type="button"
        :class="BUTTON"
        class="bg-red-700 text-white enabled:hover:bg-red-800 dark:bg-red-600 dark:enabled:hover:bg-red-500"
        :disabled="deleting"
        @click="confirm"
      >
        {{ deleting ? t("deleteActivity.deleting") : t("deleteActivity.confirm") }}
      </button>
    </div>
  </dialog>
</template>
