// result error

export class Result<T> {
  inner: T | Error;

  constructor(inner: T | Error, reason: string | undefined = undefined) {
    this.inner = inner;
    if (inner instanceof Error) {
      (this.inner as Error).message = reason;
    }
  }

  /**
   * resolve inner value or throw Error
   *
   * @return T
   */
  unwrap(): T {
    if (this.inner instanceof Error) {
      throw this.inner as Error;
    }

    return this.inner as T;
  }
}
