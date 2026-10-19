import type { ReactNode } from "react";
import { useStore } from "../store";

export function RecoveryNotice(): ReactNode {
  const { recoveryRecords, openRecovery, acknowledgeRecovery } = useStore();
  const visible = recoveryRecords.filter((record) => !record.acknowledged);
  if (visible.length === 0) return null;
  return (
    <section className="recovery-notice" aria-label="Recovery artifacts">
      <strong>Recovery files preserved</strong>
      <p>Vela kept displaced or proposed bytes. Review them before acknowledging.</p>
      <div className="recovery-list">
        {visible.map((record) => (
          <div className="recovery-record" key={record.id}>
            <span title={record.originalPath}>{record.originalPath}</span>
            <small title={record.recoveryPath}>{record.artifact}: {record.recoveryPath}</small>
            <div>
              <button onClick={() => void openRecovery(record.id)}>Open</button>
              <button onClick={() => void acknowledgeRecovery(record.id)}>Acknowledge</button>
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}
