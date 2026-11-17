export interface AmendCommitOptions {
  message: string;
  resetAuthor: boolean;
  signoff: boolean;
  removal?: { head: string; paths: string[] };
}
