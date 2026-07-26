import { AtomicMark } from "../AtomicMark/AtomicMark";
import styles from "./Brand.module.css";

export function Brand() {
  return (
    <div className={styles.brand}>
      <AtomicMark size={28} className={styles.mark} />
      <div className={styles.text}>
        <span className={styles.brandName}>Accelerator</span>
        <span className={styles.brandSub}>VISUALISER</span>
      </div>
    </div>
  );
}
