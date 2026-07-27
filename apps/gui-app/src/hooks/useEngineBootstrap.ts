import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { getEngineTransport } from "@/lib/engine/transport";
import { ENGINE_EVENT, type EngineEvent } from "@/lib/engineEvents";
import type { EngineStatus } from "@/types";
import { useEngineStore } from "@/stores/engineStore";

function reportBootstrapError(message: string) {
  console.error(message);
}

export function useEngineBootstrap(): void {
  const applyEvent = useEngineStore((state) => state.applyEvent);
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

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    listen<EngineEvent>(ENGINE_EVENT, (event) => {
      applyEvent(event.payload);
    })
      .then((dispose) => {
        unlisten = dispose;
      })
      .catch((err: unknown) => {
        reportBootstrapError(String(err));
      });

    return () => {
      unlisten?.();
    };
  }, [applyEvent]);
}
