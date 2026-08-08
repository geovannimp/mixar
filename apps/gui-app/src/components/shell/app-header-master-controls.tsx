import { HeadphoneMonitorControls } from "@/components/mixer/headphone-monitor-controls";
import { useEngineHeaderInfo } from "@/hooks/engine/use-engine-header-info";
import { StatusPill } from "./status-pill";
import { PreviewCard, PreviewCardPopup, PreviewCardTrigger } from "@/components/ui/preview-card";
export const AppHeaderMasterControls = () => {
  const { running, backend, sampleRate } = useEngineHeaderInfo();
  return (
    <div className="relative z-20 flex shrink-0 items-center gap-2.5 px-2 sm:gap-3 sm:px-3">
      <HeadphoneMonitorControls />
      <PreviewCard>
        <PreviewCardTrigger>
          <StatusPill active={running} className="cursor-default">
            {running ? "Running" : "Stopped"}
          </StatusPill>
        </PreviewCardTrigger>
        <PreviewCardPopup className={"flex-col"} sideOffset={16}>
          <div className="flex justify-between gap-3">
            <span className="text-zinc-500">Backend</span>
            <span className="font-medium text-zinc-200">{backend}</span>
          </div>
          <div className="flex justify-between gap-3">
            <span className="text-zinc-500">Sample rate</span>
            <span className="font-medium text-zinc-200">{sampleRate} Hz</span>
          </div>
        </PreviewCardPopup>
      </PreviewCard>
    </div>
  );
};
