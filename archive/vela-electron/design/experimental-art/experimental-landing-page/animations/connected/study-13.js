export default {
 name:'Corner glints',family:'Light',description:'Tiny corner glints light up only where the pearl passes close.',rule:'Every vertex uses its own distance to the pearl, creating a spatial handoff.',
 setup({path,cubeGroups,set}){const vertices=[[[504,172],[361,246],[647,246],[504,328],[361,401],[647,401],[504,484]],[[355,442],[213,519],[497,519],[355,598],[213,679],[497,679],[355,760]],[[669,442],[526,519],[812,519],[669,598],[526,679],[812,679],[669,760]]];const nodes=vertices.map((v,i)=>v.map(([x,y])=>({x,y,n:path(cubeGroups[i],`M${x-10} ${y}H${x+10}M${x} ${y-10}V${y+10}`,'#fff4ff',2.5)})));return ({i,q,strength})=>nodes[i].forEach(({x,y,n})=>{const a=Math.max(0,1-Math.hypot(q.x-x,q.y-y)/155);set(n,{opacity:strength*a*a*.85});});}
};
