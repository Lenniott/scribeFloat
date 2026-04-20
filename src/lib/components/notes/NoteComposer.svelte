<script lang="ts">
    import Button from "@components/Button.svelte";
    import Send from "lucide-svelte/icons/send";

    let {
        value = $bindable(""),
        placeholder = "Add a note…",
        disabled = false,
        onSubmit,
    }: {
        value?: string;
        placeholder?: string;
        disabled?: boolean;
        onSubmit?: () => void;
    } = $props();

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
    class="group relative flex gap-2 rounded-md border-b border-transparent bg-surface-container-high px-2 py-2 text-body-md text-on-surface transition
           focus-within:ring-primary
		   focus-within:ring-1
           focus-within:bg-surface-container-high
           aria-disabled:opacity-40
           aria-disabled:pointer-events-none
           aria-disabled:cursor-not-allowed"
>
    <textarea
        bind:value
        {placeholder}
        {disabled}
        rows="3"
        class="min-h-18 min-w-0 flex-1 resize-none bg-transparent outline-none placeholder:text-on-surface/40"
        onkeydown={onkeydown}
    ></textarea>

    <div class="flex flex-col justify-end">
        <Button
            variant="primary"
            size="normal"
            icon={Send}
            iconOnly
            aria-label="Add note"
            disabled={disabled || !value.trim()}
            onclick={submit}
        />
    </div>
</div>