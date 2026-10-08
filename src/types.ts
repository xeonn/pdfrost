export interface Warning {
  tag: string;
  location: string;
}

export interface Rendered {
  pdf: number[];
  warnings: Warning[];
}
