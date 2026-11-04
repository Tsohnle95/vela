import a from './studies-01-10.js';
import b from './studies-11-20.js';
import c from './studies-21-30.js';
import d from './studies-31-40.js';
import e from './studies-41-50.js';
import {createStage as buildStage} from './engine.js';
export {DURATION} from './engine.js';
export const concepts=[...a,...b,...c,...d,...e].map((s,i)=>({...s,id:String(i+1).padStart(2,'0')}));
export function createStage(host,id){return buildStage(host,concepts.find(c=>c.id===id)||concepts[0]);}
