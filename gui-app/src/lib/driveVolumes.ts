import type { VolumeInfo } from "../types";

export function findActiveVolume(
  volumes: VolumeInfo[],
  currentPath: string | null,
): VolumeInfo | null {
  if (!currentPath || volumes.length === 0) {
    return null;
  }

  const sorted = [...volumes].sort((a, b) => b.path.length - a.path.length);
  for (const volume of sorted) {
    if (currentPath === volume.path) {
      return volume;
    }
    if (volume.path !== "/" && currentPath.startsWith(`${volume.path}/`)) {
      return volume;
    }
    if (volume.path === "/" && currentPath.startsWith("/")) {
      return volume;
    }
  }

  return null;
}

export function isAtVolumeRoot(listingPath: string, selectedVolume: VolumeInfo | null): boolean {
  if (!selectedVolume) {
    return false;
  }
  return listingPath === selectedVolume.path;
}

export interface PathBreadcrumb {
  label: string;
  path: string;
  isCurrent: boolean;
}

export function buildPathBreadcrumbs(
  listingPath: string,
  selectedVolume: VolumeInfo | null,
): PathBreadcrumb[] {
  if (!selectedVolume) {
    return [{ label: listingPath, path: listingPath, isCurrent: true }];
  }

  const volumePath = selectedVolume.path;
  const volumeName = selectedVolume.name;

  if (listingPath === volumePath) {
    return [{ label: volumeName, path: volumePath, isCurrent: true }];
  }

  let relative = "";
  if (volumePath === "/") {
    relative = listingPath.replace(/^\//, "");
  } else if (listingPath.startsWith(`${volumePath}/`)) {
    relative = listingPath.slice(volumePath.length + 1);
  } else {
    return [{ label: listingPath, path: listingPath, isCurrent: true }];
  }

  const segments = relative.split("/").filter(Boolean);
  const crumbs: PathBreadcrumb[] = [{ label: volumeName, path: volumePath, isCurrent: false }];

  for (let index = 0; index < segments.length; index += 1) {
    const segmentPath =
      volumePath === "/"
        ? `/${segments.slice(0, index + 1).join("/")}`
        : `${volumePath}/${segments.slice(0, index + 1).join("/")}`;

    crumbs.push({
      label: segments[index],
      path: segmentPath,
      isCurrent: index === segments.length - 1,
    });
  }

  return crumbs;
}

export function splitPathBreadcrumbs(
  listingPath: string,
  selectedVolume: VolumeInfo | null,
): { ancestors: PathBreadcrumb[]; current: PathBreadcrumb } {
  const crumbs = buildPathBreadcrumbs(listingPath, selectedVolume);
  const current = crumbs.find((crumb) => crumb.isCurrent) ?? crumbs[crumbs.length - 1];
  const ancestors = crumbs.filter((crumb) => !crumb.isCurrent);

  return { ancestors, current };
}
