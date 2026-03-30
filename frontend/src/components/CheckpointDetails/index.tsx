import { useEffect, useState } from "react";
import { useSearchParams } from "react-router-dom";
import styles from "../../styles/CheckpointDetails.module.css";
import { RpcCheckpointInfoCheckpointExp } from "../../types";
import { truncateTxid } from "../../utils/lib";
import Pagination from "../Paginator/Pagination/index";
import { useConfig } from "../../hooks/useConfig";

const CheckpointDetails = () => {
  const [searchParams] = useSearchParams();
  const page = searchParams.get("p"); // Get the "p" query parameter

  // Ensure `currentPage` updates when `p` changes
  const [currentPage, setCurrentPage] = useState<number>(Number(page) || 0);
  const [checkpoint, setData] = useState<RpcCheckpointInfoCheckpointExp | null>(
    null,
  );
  const [totalPages, setTotalPages] = useState(0);
  const [firstPage, setFirstPage] = useState(0);
  const rowsPerPage = 1; // Fixed value
  const { apiBaseUrl, bitcoinExplorerBaseUrl } =
    useConfig();

  useEffect(() => {
    // Convert the query param `p` to a number
    const pageNumber = Number(page);
    if (!isNaN(pageNumber) && pageNumber !== currentPage) {
      setCurrentPage(pageNumber);
    }
  }, [page]);

  useEffect(() => {
    console.log("currentPage", currentPage);
    const fetchData = async () => {
      try {
        const response = await fetch(
          `${apiBaseUrl}/api/checkpoint?p=${currentPage}`,
        );
        const result = await response.json();
        setData(result.result.items[0]);
        console.log("result", result);
        setTotalPages(result.result.total_pages);
        setFirstPage(result.result.absolute_first_page);
      } catch (error) {
        console.error("Error fetching checkpoint data:", error);
      }
    };
    if (currentPage >= 0) fetchData();
  }, [currentPage, rowsPerPage]);

  if (!checkpoint) {
    return <div className={styles.noData}>Loading...</div>;
  }
  return (
    <>
      <div className={styles.checkpointContainer}>
        <div className={styles.checkpointRow}>
          <span className={styles.checkpointLabel}>Checkpoint index:</span>
          <span className={styles.checkpointValue}>{checkpoint.idx}</span>
        </div>
        <div className={styles.checkpointRow}>
          <span className={styles.checkpointLabel}>Checkpoint TXID:</span>
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
        </div>
        <div className={styles.checkpointRow}>
          <span className={styles.checkpointLabel}>Status:</span>
          <span className={styles.checkpointValue}>
            {checkpoint.confirmation_status}
          </span>
        </div>
        <div className={styles.checkpointRow}>
          <span className={styles.checkpointLabel}>Signet start block:</span>
          <span className={styles.checkpointValue}>
            <a
              href={`${bitcoinExplorerBaseUrl}/block/${checkpoint.l1_range[0]}`}
              target="_blank"
              rel="noreferrer"
            >
              {checkpoint.l1_range[0]}
            </a>
          </span>
        </div>
        <div className={styles.checkpointRow}>
          <span className={styles.checkpointLabel}>Signet end block:</span>
          <span className={styles.checkpointValue}>
            <a
              href={`${bitcoinExplorerBaseUrl}/block/${checkpoint.l1_range[1]}`}
              target="_blank"
              rel="noreferrer"
            >
              {checkpoint.l1_range[1]}
            </a>
          </span>
        </div>
        <div className={styles.checkpointRow}>
          <span className={styles.checkpointLabel}>Strata start block:</span>
          <span className={styles.checkpointValue}>
            {checkpoint.l2_range[0]}
          </span>
        </div>
        <div className={styles.checkpointRow}>
          <span className={styles.checkpointLabel}>Strata end block:</span>
          <span className={styles.checkpointValue}>
            {checkpoint.l2_range[1]}
          </span>
        </div>
      </div>

      <Pagination
        currentPage={currentPage}
        firstPage={firstPage}
        totalPages={totalPages}
        setPage={setCurrentPage}
      />
    </>
  );
};

export default CheckpointDetails;
