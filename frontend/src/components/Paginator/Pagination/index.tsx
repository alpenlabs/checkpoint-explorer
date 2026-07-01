import React, { useState } from "react";
// import { useAlert } from "../../../hooks/useAlert";
import styles from "../../../styles/Pagination.module.css";
import AlertComponent from "../../Alert";
interface PaginationProps {
  currentPage: number;
  firstPage: number;
  totalPages: number;
  setPage: (page: number) => void;
}

const Pagination: React.FC<PaginationProps> = ({
  currentPage,
  firstPage,
  totalPages,
  setPage,
}) => {
  const pageWindowSize = 1;
  const startPage = Math.max(
    firstPage,
    currentPage - Math.floor(pageWindowSize / 2),
  );
  const endPage = Math.min(totalPages, startPage + pageWindowSize - 1);

  const [editablePage, setEditablePage] = useState<string | null>(null);
  const [showAlert, setShowAlert] = useState(false);
  const pageInputValue = editablePage ?? String(currentPage);

  // Function to update both state and URL
  const updatePage = (page: number) => {
    if (page >= firstPage && page <= totalPages) {
      setPage(page);
      setEditablePage(null);
    } else {
      setShowAlert(true);
      setTimeout(() => {
        // wait for 2 seconds and then set the alert to false
        setShowAlert(false);
      }, 2000);
      return;
    }
  };

  // Validate and navigate to new page on input change
  const handlePageChange = () => {
    const page = Number(pageInputValue);
    updatePage(page);
  };

  return (
    <>
      <div className={styles.footer}>
        <div className={styles.pageButtons}>
          {/* First Button */}
          <button
            className={`${styles.pageButton} ${currentPage === firstPage ? styles.disabled : ""}`}
            onClick={() => updatePage(firstPage)}
            disabled={currentPage === firstPage}
          >
            «
          </button>

          {/* Previous Button */}
          <button
            className={`${styles.pageButton} ${currentPage === firstPage ? styles.disabled : ""}`}
            onClick={() => updatePage(currentPage - 1)}
            disabled={currentPage === firstPage}
          >
            ‹
          </button>

          {/* Page Buttons */}
          {Array.from({ length: endPage - startPage + 1 }, (_, index) => {
            const page = startPage + index;
            return page === currentPage ? (
              <input
                key={page}
                className={styles.pageInput}
                value={pageInputValue}
                onChange={(e) => setEditablePage(e.target.value)}
                onBlur={handlePageChange}
                onKeyDown={(e) => e.key === "Enter" && handlePageChange()}
              />
            ) : (
              <button
                key={page}
                className={styles.pageButton}
                onClick={() => updatePage(page)}
              >
                {page}
              </button>
            );
          })}

          {/* Next Button */}
          <button
            className={`${styles.pageButton} ${currentPage === totalPages ? styles.disabled : ""}`}
            onClick={() => updatePage(currentPage + 1)}
            disabled={currentPage === totalPages}
          >
            ›
          </button>

          {/* Last Button */}
          <button
            className={`${styles.pageButton} ${currentPage === totalPages ? styles.disabled : ""}`}
            onClick={() => updatePage(totalPages)}
            disabled={currentPage === totalPages}
          >
            »
          </button>
        </div>
        {totalPages > 0 && (
          <div className={styles.pageInfo}>
            Page {currentPage} of {totalPages}
          </div>
        )}
        {totalPages === 0 && (
          <div className={styles.pageInfo}>No data found</div>
        )}
      </div>
      <div className={styles.alertWrapper}>
        {showAlert && <AlertComponent />}
      </div>
    </>
  );
};

export default Pagination;
