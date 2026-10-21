import type { CSSProperties } from "react";
export type HighlightRequest = {type:"highlight";key:string;id:number;code:string;language:string} | {type:"dispose";key:string};
export type HighlightResult = {key:string;id:number;code:string;language:string;lines:Array<Array<{text:string;style:CSSProperties}>> | null};
