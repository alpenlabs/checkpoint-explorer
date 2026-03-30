import { useEffect, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { RpcCheckpointInfoCheckpointExp } from "../../../types";
import { truncateTxid } from "../../../utils/lib";
import Pagination from "../../Paginator/Pagination";
import styles from "../../../styles/Table.module.css";
import { useConfig } from "../../../hooks/useConfig";

const TableBody: React.FC = () => {
  const [data, setData] = useState<RpcCheckpointInfoCheckpointExp[]>([]);
  const [rowsPerPage] = useState(10); // Fixed value
  const [totalPages, setTotalPages] = useState(0);
  const [firstPage, setFirstPage] = useState(1);
  const [searchParams, setSearchParams] = useSearchParams();

  // Get `p` from URL and ensure it's a valid number
  const pageFromUrl = Number(searchParams.get("p")) || 1;
  const [currentPage, setCurrentPage] = useState(pageFromUrl);
  const {
    apiBaseUrl,
    bitcoinExplorerBaseUrl,
    refreshIntervalS,
  } = useConfig();

  useEffect(() => {
    if (currentPage !== pageFromUrl) {
      setCurrentPage(pageFromUrl);
    }
  }, [pageFromUrl]);

  /**
   * - Ensures data reloads when the user changes pages.
   */
  useEffect(() => {
    const fetchData = async () => {
      try {
        console.log("fetching data...");
        const response = await fetch(
          `${apiBaseUrl}/api/checkpoints?p=${currentPage}&ps=${rowsPerPage}`,
        );
        const result = await response.json();
        setData(result.result.items);
        setTotalPages(result.result.total_pages);
        setFirstPage(result.result.absolute_first_page);
      } catch (error) {
        console.error("Error fetching data:", error);
      }
    };

    fetchData();

    // convert refreshIntervalS to ms
    const interval = setInterval(fetchData, refreshIntervalS * 1000);

    return () => clearInterval(interval); // Clear interval when unmounting or dependencies change
  }, [currentPage, rowsPerPage]); // Trigger fetch when `currentPage` changes

  /**
   * Immediately update `searchParams` and state
   * - Prevents the need for a second click.
   */
  const setPage = (page: number) => {
    if (page < firstPage || page > totalPages || page === currentPage) return;

    setSearchParams({ p: page.toString() }); // Update URL first
    setCurrentPage(page); // Then update state immediately
  };

  return (
    <>
      <div className={styles.tableContainer}>
        <table className={styles.table}>
          <thead className={styles.tableRowHeader}>
            <tr>
              <th className={styles.tableHeader}>Checkpoint index</th>
              <th className={styles.tableHeader}>Checkpoint TXID</th>
              <th className={styles.tableHeader}>Status</th>
              <th className={styles.tableHeader}>Signet start block</th>
              <th className={styles.tableHeader}>Signet end block</th>
              <th className={styles.tableHeader}>Strata start block</th>
              <th className={styles.tableHeader}>Strata end block</th>
            </tr>
          </thead>
          <tbody>
            {data.map((checkpoint) => (
              <tr className={styles.tableRowItems} key={checkpoint.idx}>
                <td className={styles.tableCell}>
                  <Link to={`/checkpoint?p=${checkpoint.idx}`}>
                    {checkpoint.idx}
                  </Link>
                </td>
                <td
                  className={styles.tableCell}
                  title={checkpoint.l1_reference?.txid}
                >
                  {checkpoint.l1_reference &&
                  checkpoint.l1_reference.txid &&
                  checkpoint.l1_reference.txid !== "N/A" &&
                  checkpoint.l1_reference.txid !== "-" ? (
                    <a
                      href={`${bitcoinExplorerBaseUrl}/tx/${checkpoint.l1_reference?.txid}`}
                      target="_blank"
                      rel="noreferrer"
                    >
                      {truncateTxid(checkpoint.l1_reference?.txid)}
                    </a>
                  ) : (
                    checkpoint.l1_reference?.txid
                  )}
                </td>
                <td className={styles.tableCell}>
                  {checkpoint.confirmation_status}
                </td>
                <td className={styles.tableCell}>
                  <a
                    href={`${bitcoinExplorerBaseUrl}/block/${checkpoint.l1_range[0]}`}
                    target="_blank"
                    rel="noreferrer"
                  >
                    {checkpoint.l1_range[0]}
                  </a>
                </td>
                <td className={styles.tableCell}>
                  <a
                    href={`${bitcoinExplorerBaseUrl}/block/${checkpoint.l1_range[1]}`}
                    target="_blank"
                    rel="noreferrer"
                  >
                    {checkpoint.l1_range[1]}
                  </a>
                </td>
                <td className={styles.tableCell}>
                  {checkpoint.l2_range[0]}
                </td>
                <td className={styles.tableCell}>
                  {checkpoint.l2_range[1]}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {totalPages == 0 && <div className={styles.noData}>Loading...</div>}
      {totalPages > 0 && (
        <Pagination
          currentPage={currentPage}
          firstPage={firstPage}
          totalPages={totalPages}
          setPage={setPage}
        />
      )}
    </>
  );
};

export default TableBody;
