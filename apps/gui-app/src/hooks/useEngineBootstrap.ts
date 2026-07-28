import { invoke } from "@tauri-apps/api/core";
import { useEffect } from "react";
import { getEngineTransport } from "@/lib/engine/transport";
import type { EngineStatus } from "@/types";
import { useEngineStore } from "@/stores/engineStore";

function reportBootstrapError(message: string) {
  console.error(message);
}

export function useEngineBootstrap(): void {
  const applyBusBytes = useEngineStore((state) => state.applyBusBytes);
  const setStatus = useEngineStore((state) => state.setStatus);

  useEffect(() => {
    invoke<EngineStatus>("get_status")
      .then((status) => {
        setStatus(status);
      })
      .catch((err: unknown) => {
        reportBootstrapError(String(err));
      });
  }, [setStatus]);

  useEffect(() => {
    const transport = getEngineTransport();
    return transport.subscribe((bytes) => {
      applyBusBytes(bytes);
    });
  }, [applyBusBytes]);
}
