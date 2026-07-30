import { useFormContext } from "@/components/settings/form-context";
import { buttonBase } from "@/lib/ui";

interface SettingsSaveButtonProps {
  busy: boolean;
}

export function SettingsSaveButton({ busy }: SettingsSaveButtonProps) {
  const form = useFormContext();

  return (
    <form.Subscribe selector={(state) => state.isSubmitting}>
      {(isSubmitting) => {
        const disabled = busy || isSubmitting;
        return (
          <button
            type="submit"
            className={`${buttonBase} border-emerald-500/45 bg-emerald-500/15 hover:bg-emerald-500/25`}
            disabled={disabled}
          >
            {disabled ? "Saving…" : "Save"}
          </button>
        );
      }}
    </form.Subscribe>
  );
}
