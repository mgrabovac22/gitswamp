import { callTauri } from "./gitCall";
import { secretDetectorEnabled } from "@/shared/config/secretDetectorPreferences";
import { requestSecretConfirmation, type SecretScan } from "@/shared/notifications/secretConfirmation";

export async function checkCommitSecrets(path: string, isCurrent: () => boolean, refreshStatus: () => Promise<void>): Promise<{ secretCheck: boolean; secretApproval?: string } | null> {
  const enabled = secretDetectorEnabled();
  const report = await callTauri<SecretScan>("scan_secrets", { path, staged: true, includeSecrets: enabled });
  if (!isCurrent()) return null;
  if (report.findings.length || report.skipped.length || report.large_files?.length) {
    const choice = await requestSecretConfirmation(path, report);
    if (!isCurrent() || choice === "cancel") return null;
    if (choice === "ignore") {
      const paths = [...new Set(report.findings.filter(f => !f.tracked).map(f => f.path))];
      await callTauri("ignore_secret_files", { path, paths, snapshot: report.snapshot });
      if (isCurrent()) await refreshStatus();
      return null;
    }
  }
  return { secretCheck: enabled, secretApproval: report.snapshot };
}
