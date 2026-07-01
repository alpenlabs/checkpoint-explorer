// @/src/hooks/useTable.js

import { useMemo } from "react";

const calculateRange = <T>(data: T[], rowsPerPage: number): number[] => {
  const range: number[] = [];
  const num = Math.ceil(data.length / rowsPerPage);
  for (let i = 1; i <= num; i++) {
    range.push(i);
  }
  return range;
};

const sliceData = <T>(data: T[], page: number, rowsPerPage: number): T[] => {
  return data.slice((page - 1) * rowsPerPage, page * rowsPerPage);
};

const useTable = <T>(data: T[], page: number, rowsPerPage: number) => {
  const tableRange = useMemo(
    () => calculateRange(data, rowsPerPage),
    [data, rowsPerPage],
  );
  const slice = useMemo(
    () => sliceData(data, page, rowsPerPage),
    [data, page, rowsPerPage],
  );

  return { slice, range: tableRange };
};

export default useTable;
