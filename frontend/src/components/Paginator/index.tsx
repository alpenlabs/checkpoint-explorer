import { Route, Routes } from "react-router-dom";
import styles from "../../styles/App.module.css";
import CheckpointDetails from "../CheckpointDetails";
import TableBody from "../Table/TableBody";
// Define the props for the Table component

const PaginatedData = () => {
  return (
    <div className={styles.wrapper}>
      <Routes>
        <Route
          path="/"
          element={
            <>
              <TableBody />
            </>
          }
        />
        <Route
          path="/checkpoint"
          element={
            <>
              <CheckpointDetails />
            </>
          }
        />
        <Route
          path="/search"
          element={
            <>
              <CheckpointDetails />
            </>
          }
        />
      </Routes>
    </div>
  );
};

export default PaginatedData;
