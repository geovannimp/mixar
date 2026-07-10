import {
  compareItems,
  rankItem,
  type RankingInfo,
} from "@tanstack/match-sorter-utils";
import {
  sortingFns,
  type FilterFn,
  type SortingFn,
} from "@tanstack/react-table";
import type { LibraryTableRow } from "../types";
import { libraryRowSearchText } from "./libraryTable";

declare module "@tanstack/react-table" {
  interface FilterFns {
    fuzzy: FilterFn<LibraryTableRow>;
  }
  interface FilterMeta {
    itemRank: RankingInfo;
  }
}

export const fuzzyFilter: FilterFn<LibraryTableRow> = (
  row,
  columnId,
  value,
  addMeta,
) => {
  const itemRank = rankItem(row.getValue(columnId), value);
  addMeta({ itemRank });
  return itemRank.passed;
};

export const fuzzySort: SortingFn<LibraryTableRow> = (rowA, rowB, columnId) => {
  let direction = 0;

  if (rowA.columnFiltersMeta[columnId]) {
    direction = compareItems(
      rowA.columnFiltersMeta[columnId]?.itemRank!,
      rowB.columnFiltersMeta[columnId]?.itemRank!,
    );
  }

  return direction === 0
    ? sortingFns.alphanumeric(rowA, rowB, columnId)
    : direction;
};

export const libraryGlobalFilter: FilterFn<LibraryTableRow> = (
  row,
  _columnId,
  value,
  addMeta,
) => {
  const itemRank = rankItem(libraryRowSearchText(row.original), value);
  addMeta({ itemRank });
  return itemRank.passed;
};
