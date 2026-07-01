import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useConfig } from "../hooks/useConfig";
import styles from "../styles/SearchSection.module.css";

const SearchSection = () => {
  const [query, setQuery] = useState("");
  const [error, setError] = useState(false); // State to track error visibility
  const navigate = useNavigate();
  const { apiBaseUrl } = useConfig();

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!query.trim()) return;

    try {
      const response = await fetch(
        `${apiBaseUrl}/api/search?query=${query.trim()}&ps=1`,
      );
      const result = await response.json();

      if (result.error) {
        setError(true); // Show error message
        setTimeout(() => setError(false), 3000); // Hide after 3s
        return;
      }

      const checkpoint_id = result.result;
      if (checkpoint_id >= 0) {
        navigate(`/checkpoint?p=${checkpoint_id}`);
      }
    } catch (error) {
      console.error("UNKNOWN Error fetching data:", error);
    }
  };

  return (
    <div className={styles.searchSection}>
      <>
        <Link to="/" className={styles.title}>
          <h1>Checkpoint explorer</h1>
        </Link>
        <form onSubmit={handleSearch} className={styles.searchBox}>
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="🔍 Search by orchestration layer block number or block hash"
            className={styles.searchInput}
            required
          />
          {/* Dynamically apply the visible class */}
          <div
            className={`${styles.errorMessage} ${error ? styles.visible : ""}`}
          >
            Invalid search entry
          </div>
        </form>
      </>
    </div>
  );
};

export default SearchSection;
