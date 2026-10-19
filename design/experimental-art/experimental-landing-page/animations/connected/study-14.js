export default {
 name:'Edge reflection',family:'Surface',description:'A short reflected streak follows the nearest point on each cube’s perimeter.',rule:'The edge segment tracks the pearl’s bearing around the cube, rather than a separate clock.',
 setup({path,cubeGroups,faces,set}){
 const nodes=faces.map((d,i)=>{const n=path(cubeGroups[i],d,'#f3e6ff',4);const length=n.getTotalLength();const samples=Array.from({length:181},(_,k)=>{const at=k*length/180,p=n.getPointAtLength(at);return {at,x:p.x,y:p.y};});set(n,{'stroke-dasharray':`48 ${length-48}`});return {n,length,samples};});
 return ({i,q,strength})=>{const {n,samples}=nodes[i];let nearest=samples[0];for(const a of samples)if(Math.hypot(a.x-q.x,a.y-q.y)<Math.hypot(nearest.x-q.x,nearest.y-q.y))nearest=a;set(n,{'stroke-dashoffset':24-nearest.at,opacity:strength*.55});};
 }
};
