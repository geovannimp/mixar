import { HardDrive, Usb } from "lucide-react";
import {
  Select,
  SelectItem,
  SelectPopup,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { VolumeInfo } from "../types";

type VolumeOption = {
  label: string;
  value: string;
  volume: VolumeInfo;
};

interface DriveSelectorProps {
  volumes: VolumeInfo[];
  selectedVolume: VolumeInfo | null;
  disabled?: boolean;
  onSelectVolume: (path: string) => void;
}

function VolumeIcon({ volume }: { volume: VolumeInfo }) {
  if (volume.is_removable) {
    return <Usb className="size-4 shrink-0 text-amber-400/80" aria-hidden />;
  }
  return <HardDrive className="size-4 shrink-0 text-zinc-500" aria-hidden />;
}

function VolumeLine({ volume }: { volume: VolumeInfo }) {
  return (
    <span className="flex min-w-0 items-center gap-2">
      <VolumeIcon volume={volume} />
      <span className="truncate text-sm">{volume.name}</span>
    </span>
  );
}

function toVolumeOption(volume: VolumeInfo): VolumeOption {
  return {
    label: volume.name,
    value: volume.path,
    volume,
  };
}

export function DriveVolumeList({
  volumes,
  busy,
  onSelectVolume,
}: {
  volumes: VolumeInfo[];
  busy: boolean;
  onSelectVolume: (path: string) => void;
}) {
  if (volumes.length === 0) {
    return (
      <p className="rounded border border-dashed border-white/10 px-3 py-6 text-sm text-zinc-500">
        No drives found.
      </p>
    );
  }

  return (
    <ul className="flex flex-col gap-0.5">
      {volumes.map((volume) => (
        <li key={volume.path}>
          <button
            type="button"
            className="w-full rounded border-l-2 border-l-transparent px-3 py-2 text-left hover:bg-white/5"
            disabled={busy}
            title={volume.path}
            onClick={() => onSelectVolume(volume.path)}
          >
            <span className="flex items-start gap-2">
              <VolumeIcon volume={volume} />
              <span className="min-w-0 flex-1">
                <span className="block truncate text-sm font-medium">{volume.name}</span>
                <span className="mt-0.5 block truncate text-xs text-zinc-500">{volume.path}</span>
              </span>
            </span>
          </button>
        </li>
      ))}
    </ul>
  );
}

export function DriveSelector({
  volumes,
  selectedVolume,
  disabled = false,
  onSelectVolume,
}: DriveSelectorProps) {
  const options = volumes.map(toVolumeOption);
  const selected = selectedVolume ? toVolumeOption(selectedVolume) : null;

  if (options.length === 0) {
    return null;
  }

  return (
    <Select
      aria-label="Select drive"
      disabled={disabled}
      value={selected}
      onValueChange={(item) => {
        if (!item) {
          return;
        }
        onSelectVolume(item.value);
      }}
      itemToStringValue={(item) => item.value}
    >
      <SelectTrigger
        className="h-8 min-h-8 border-white/10 bg-zinc-900/80 py-0 text-sm shadow-none"
        title={selectedVolume?.path}
      >
        <SelectValue>
          {(item) =>
            item ? (
              <VolumeLine volume={item.volume} />
            ) : (
              <span className="text-zinc-500">Select drive</span>
            )
          }
        </SelectValue>
      </SelectTrigger>
      <SelectPopup>
        {options.map((option) => (
          <SelectItem key={option.value} value={option} title={option.volume.path}>
            <VolumeLine volume={option.volume} />
          </SelectItem>
        ))}
      </SelectPopup>
    </Select>
  );
}
