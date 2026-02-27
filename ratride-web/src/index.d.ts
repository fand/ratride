export interface RatrideConfig {
  parent?: HTMLElement;
  fontSize?: number;
  theme?: string;
  fonts?: Record<string, string>;
}

export interface RatrideInstance {
  destroy(): void;
}

export function run(
  md: string,
  config?: RatrideConfig,
): Promise<RatrideInstance>;
