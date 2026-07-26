/// <reference types="vite/client" />

declare module "*.hex?raw" {
  const content: string;
  export default content;
}
