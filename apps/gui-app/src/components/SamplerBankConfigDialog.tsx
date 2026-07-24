import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogPanel,
  DialogPopup,
  DialogTitle,
} from "@/components/ui/dialog";
import { Field, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectItem,
  SelectPopup,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { SamplerBankInfo, SamplerPlayMode } from "@/types";

type BankPlayModeValue = "default" | SamplerPlayMode;

const PLAY_MODE_OPTIONS: { value: BankPlayModeValue; label: string }[] = [
  { value: "default", label: "Default (inherit settings)" },
  { value: "oneshot", label: "Oneshot" },
  { value: "hold", label: "Hold" },
  { value: "loop", label: "Loop" },
];

function playModeValue(mode: SamplerPlayMode | null | undefined): BankPlayModeValue {
  return mode == null ? "default" : mode;
}

function playModeFromValue(value: BankPlayModeValue): SamplerPlayMode | null {
  return value === "default" ? null : value;
}

interface SamplerBankConfigDialogProps {
  open: boolean;
  bank: SamplerBankInfo | null;
  onOpenChange: (open: boolean) => void;
  onSave: (bankId: string, name: string, playMode: SamplerPlayMode | null) => void;
}

export function SamplerBankConfigDialog({
  open,
  bank,
  onOpenChange,
  onSave,
}: SamplerBankConfigDialogProps) {
  const [name, setName] = useState("");
  const [playMode, setPlayMode] = useState<BankPlayModeValue>("default");

  useEffect(() => {
    if (!open || !bank) {
      return;
    }
    setName(bank.name);
    setPlayMode(playModeValue(bank.play_mode));
  }, [bank, open]);

  const selectedPlayMode =
    PLAY_MODE_OPTIONS.find((option) => option.value === playMode) ?? PLAY_MODE_OPTIONS[0];

  const trimmedName = name.trim();
  const isDirty =
    Boolean(bank) &&
    (trimmedName !== (bank?.name ?? "") || playMode !== playModeValue(bank?.play_mode));
  const canSave = Boolean(bank && trimmedName && isDirty);

  const commit = () => {
    if (!bank || !canSave) {
      return;
    }
    onSave(bank.id, trimmedName, playModeFromValue(playMode));
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogPopup className="max-w-sm">
        <DialogHeader>
          <DialogTitle>Bank settings</DialogTitle>
          <DialogDescription>Rename this sampler bank and choose its play mode.</DialogDescription>
        </DialogHeader>
        <DialogPanel>
          <div className="space-y-4">
            <Field>
              <FieldLabel htmlFor="sampler-bank-name">Name</FieldLabel>
              <Input
                id="sampler-bank-name"
                value={name}
                autoFocus
                onChange={(event) => setName(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key !== "Enter") {
                    return;
                  }
                  event.preventDefault();
                  commit();
                }}
              />
            </Field>
            <Field>
              <FieldLabel>Play mode</FieldLabel>
              <Select
                aria-label="Bank play mode"
                value={selectedPlayMode}
                onValueChange={(item) => {
                  if (!item) {
                    return;
                  }
                  setPlayMode(item.value);
                }}
                itemToStringValue={(item) => item.value}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectPopup>
                  {PLAY_MODE_OPTIONS.map((option) => (
                    <SelectItem key={option.value} value={option}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectPopup>
              </Select>
            </Field>
          </div>
        </DialogPanel>
        <DialogFooter>
          <DialogClose render={<Button variant="outline" />}>Cancel</DialogClose>
          <Button disabled={!canSave} onClick={commit}>
            Save
          </Button>
        </DialogFooter>
      </DialogPopup>
    </Dialog>
  );
}
