import { useLocation, useNavigate } from "react-router-dom";
import { Tabs, TabsList, TabsTab } from "@/components/ui/tabs";
import { useMemo } from "react";

type Tabs = "mixer" | "settings";
const TabPathMapping = new Map<Tabs, string>([
  ["mixer", "/"],
  ["settings", "/settings"],
]);

export const AppHeaderTabs = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const tab = useMemo(
    () => TabPathMapping.entries().find(([_, path]) => location.pathname === path)?.[0],
    [location.pathname],
  );

  return (
    <Tabs
      value={tab}
      className="gap-0"
      onValueChange={(value) => {
        if (Array.from(TabPathMapping.keys()).includes(value)) {
          void navigate(TabPathMapping.get(value) ?? "/");
        }
      }}
    >
      <TabsList className="h-6">
        {TabPathMapping.keys().map((key) => (
          <TabsTab key={key} value={key} className="uppercase tracking-wide sm:h-5 sm:text-sm">
            {key}
          </TabsTab>
        ))}
      </TabsList>
    </Tabs>
  );
};
