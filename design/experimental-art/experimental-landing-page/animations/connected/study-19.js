export default {
 name:'Frosted grain',family:'Particles',description:'A few fixed grains catch the pearl’s light as it moves past the surface.',rule:'The grains stay anchored; only grains near the pearl’s reflection become visible.',
 setup({circle,faceEffects,set,centers}){const nodes=faceEffects.map((g,i)=>{const [x,y]=centers[i];return Array.from({length:35},(_,k)=>{const px=x-125+(k%7)*40,py=y-125+Math.floor(k/7)*57;return {x:px,y:py,n:circle(g,2.5,'#f6efff',{cx:px,cy:py})};});});return ({i,x,y,dx,dy,strength})=>nodes[i].forEach(v=>{const d=Math.hypot(v.x-x-dx*.4,v.y-y-dy*.4),a=Math.max(0,1-d/85);set(v.n,{opacity:a*a*strength*.7});});}
};
