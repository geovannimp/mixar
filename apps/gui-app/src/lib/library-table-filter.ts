import { compareItems, rankItem, type RankingInfo } from "@tanstack/match-sorter-utils";
import { sortingFns, type FilterFn, type SortingFn } from "@tanstack/react-table";
import type { LibraryTableRow } from "@/types";
import { libraryRowSearchText } from "./library-table";

declare module "@tanstack/react-table" {
  interface FilterFns {
    fuzzy: FilterFn<LibraryTableRow>;
  }
  interface FilterMeta {
    itemRank: RankingInfo;
  }
}

export const fuzzyFilter: FilterFn<LibraryTableRow> = (row, columnId, value, addMeta) => {
  const itemRank = rankItem(row.getValue(columnId), value);
  addMeta({ itemRank });
  return itemRank.passed;
};

export const fuzzySort: SortingFn<LibraryTableRow> = (rowA, rowB, columnId) => {
  let direction = 0;

  const rankA = rowA.columnFiltersMeta[columnId]?.itemRank;
  const rankB = rowB.columnFiltersMeta[columnId]?.itemRank;
  if (rankA && rankB) {
    direction = compareItems(rankA, rankB);
  }

  return direction === 0 ? sortingFns.alphanumeric(rowA, rowB, columnId) : direction;
};

export const libraryGlobalFilter: FilterFn<LibraryTableRow> = (row, _columnId, value, addMeta) => {
  const itemRank = rankItem(libraryRowSearchText(row.original), value);
  addMeta({ itemRank });
  return itemRank.passed;
};
