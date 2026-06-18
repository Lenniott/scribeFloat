<script lang="ts">
    import IconButton from "@components/controls/IconButton.svelte";
    import Send from "lucide-svelte/icons/send";

    let {
        value = $bindable(""),
        placeholder = "Add a note…",
        disabled = false,
        focusOnMount = false,
        onSubmit,
    }: {
        value?: string;
        placeholder?: string;
        disabled?: boolean;
        focusOnMount?: boolean;
        onSubmit?: () => void;
    } = $props();

    let textareaEl = $state<HTMLTextAreaElement | undefined>();
    let _focused = false;

    $effect(() => {
        if (focusOnMount && textareaEl && !_focused) {
            _focused = true;
            textareaEl.focus();
        }
    });

    function submit() {
        const t = value.trim();
        if (!t || disabled) return;
        onSubmit?.();
    }

    function onkeydown(e: KeyboardEvent) {
        if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            submit();
        }
    }
</script>

<div
    aria-disabled={disabled}
    class="group relative flex gap-2 rounded-md border-b border-fill bg-fill px-2 py-2 sf-body-md text-fg transition
           aria-disabled:opacity-40
           aria-disabled:pointer-events-none
           aria-disabled:cursor-not-allowed
           focus-within:ring-2 focus-within:ring-focus focus-within:ring-offset-2 focus-within:ring-offset-canvas
           "
>
    <textarea
        bind:this={textareaEl}
        bind:value
        {placeholder}
        {disabled}
        rows="3"
        class="min-h-18 min-w-0 flex-1 resize-none bg-transparent outline-none placeholder:text-fg-dim outline-0 ring-0 ring-offset-0"
        onkeydown={onkeydown}
    ></textarea>

    <div class="flex flex-col justify-end">
        <IconButton
            variant="primary"
            size="normal"
            icon={Send}
            aria-label="Add note"
            disabled={disabled || !value.trim()}
            onclick={submit}
            class="outline-0 ring-0 ring-offset-0"
        />
    </div>
</div>
