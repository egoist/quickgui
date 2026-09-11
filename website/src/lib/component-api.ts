export interface ApiEntry {
  name: string;
  type: string;
  description: string;
  source: string;
  default?: string;
  binding?: string;
}
export interface ApiSection {
  name: string;
  signature: string;
  description: string;
  source: string;
  entries: ApiEntry[];
}
export interface ComponentApi {
  sections: ApiSection[];
  example: string;
  language: "go" | "tsx" | "rust";
}
