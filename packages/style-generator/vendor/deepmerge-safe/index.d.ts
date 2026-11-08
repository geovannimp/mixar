declare function deepmerge<T>(
  target: Partial<T>,
  source: Partial<T>,
  options?: {
    arrayMerge?: (target: unknown[], source: unknown[], options?: unknown) => unknown[];
    isMergeableObject?: (value: unknown) => boolean;
    clone?: boolean;
  }
): T;

declare namespace deepmerge {
  function all<T>(objects: Partial<T>[], options?: Parameters<typeof deepmerge>[2]): T;
}

export default deepmerge;
