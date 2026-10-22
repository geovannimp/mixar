import { execFile } from 'node:child_process';

export interface ExecOptions {
  cwd: string;
  env: NodeJS.ProcessEnv;
  timeout: number;
  maxBuffer: number;
  shell: false;
}

export interface ExecResult {
  stdout: string;
  stderr: string;
}

/**
 * Runs a command without a shell and captures its output. A non-zero exit
 * rejects with the exit code and stderr attached, so callers can report both.
 */
export function execFileText(
  cmd: string,
  args: string[],
  options: ExecOptions,
): Promise<ExecResult> {
  const { promise, resolve, reject } = Promise.withResolvers<ExecResult>();
  execFile(cmd, args, { ...options, encoding: 'utf8' }, (error, stdout, stderr) => {
    if (error) {
      (error as Error & { stderr?: string }).stderr = String(stderr ?? '');
      reject(error);
      return;
    }
    resolve({ stdout: String(stdout ?? ''), stderr: String(stderr ?? '') });
  });
  return promise;
}
