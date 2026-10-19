export default {
 name:'Facing rims',family:'Light',description:'Only the cube edges facing the pearl catch a faint rim light.',rule:'Each rim’s brightness follows its outward direction relative to the pearl.',
 setup({path,cubeGroups,set,centers}){
  const rims=centers.map(([x,y],i)=>{const v=i===0?[[361,246],[504,172],[647,246],[647,401],[504,484],[361,401]]:i===1?[[213,519],[355,442],[497,519],[497,679],[355,760],[213,679]]:[[526,519],[669,442],[812,519],[812,679],[669,760],[526,679]];return v.map((a,j)=>{const b=v[(j+1)%6],mx=(a[0]+b[0])/2-x,my=(a[1]+b[1])/2-y,l=Math.hypot(mx,my);return {n:path(cubeGroups[i],`M${a}L${b}`,'#f0e8ff',4),nx:mx/l,ny:my/l};});});
  return ({i,nx,ny,strength})=>rims[i].forEach(r=>set(r.n,{opacity:strength*Math.max(0,r.nx*nx+r.ny*ny)*.65}));
 }
};
