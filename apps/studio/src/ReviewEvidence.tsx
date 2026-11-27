import type { Note, ReviewMetadata } from "./types";
export default function ReviewEvidence({
  review,
  freshness,
  snapshot,
}: {
  review: ReviewMetadata;
  freshness?: Note["freshness"];
  snapshot?: Note["review_snapshot"];
}) {
  return (
    <section className="inspector-section review-evidence">
      <h3>Review evidence · historical</h3>
      <p className="overview-note">
        Supplied review evidence. Studio has not run checks or inspected the
        repository.
      </p>
      {snapshot ? (
        <>
          <p className="status-tag historical">
            {freshness === "current"
              ? "Matching supplied head"
              : freshness === "stale"
                ? "Historical head"
                : "Head not supplied"}{" "}
            · loaded snapshot comparison
          </p>
          <p className="overview-note">
            Supplied head for this snapshot:{" "}
            {snapshot.current_head ?? "None supplied"}. This comparison belongs
            to the loaded history snapshot; it is not a current repository
            verification.
          </p>
          <p className="overview-note">{snapshot.notice}</p>
        </>
      ) : null}
      <dl className="evidence-meta">
        <dt>Repository</dt>
        <dd>{review.repository}</dd>
        <dt>Base commit</dt>
        <dd>
          <code>{review.base_commit}</code>
        </dd>
        <dt>Head commit</dt>
        <dd>
          <code>{review.head_commit}</code>
        </dd>
        <dt>Previous review</dt>
        <dd>{review.previous_review_id ?? "First review"}</dd>
        <dt>Revised by</dt>
        <dd>
          {review.revised_by === undefined
            ? "Revision links require a loaded history snapshot"
            : (review.revised_by ?? "No revision in this snapshot")}
        </dd>
      </dl>
      <h4>Recorded checks</h4>
      {review.checks.length ? (
        <ul className="review-checks">
          {review.checks.map((check, i) => (
            <li key={i}>
              <strong>{check.name}</strong>
              <span className="status-tag historical">
                {check.outcome.replaceAll("_", " ")}
              </span>
              <p className="record-text">{check.result}</p>
            </li>
          ))}
        </ul>
      ) : (
        <p>No checks recorded.</p>
      )}
      <h4>Recorded findings</h4>
      {review.findings.length ? (
        review.findings.map((finding) => (
          <article className="review-finding" key={finding.id}>
            <div className="record-meta">
              <span>{finding.id}</span>
              <span>{finding.severity}</span>
              <span className="status-tag historical">
                {finding.status.replaceAll("_", " ")}
              </span>
            </div>
            <h4>{finding.title}</h4>
            <p className="source-item">
              {finding.path}:{finding.line}
            </p>
            <p className="record-text">
              <strong>Evidence: </strong>
              {finding.evidence}
            </p>
            <p className="record-text">
              <strong>Correction: </strong>
              {finding.correction}
            </p>
          </article>
        ))
      ) : (
        <p>No findings recorded. This is not a safety verdict.</p>
      )}
    </section>
  );
}
