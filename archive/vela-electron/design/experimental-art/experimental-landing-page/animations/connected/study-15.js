export default {
 name:'Facet lighting',family:'Light',description:'Top, left, and right facets brighten separately as the pearl moves around them.',rule:'Each face receives light from its own facing direction, weighted by proximity.',
 setup({add,faceEffects,set,tops}){const left=['M361 246L504 328V484L361 401Z','M213 519L355 598V760L213 679Z','M526 519L669 598V760L526 679Z'],right=['M504 328L647 246V401L504 484Z','M355 598L497 519V679L355 760Z','M669 598L812 519V679L669 760Z'];const nodes=faceEffects.map((g,i)=>[tops[i],left[i],right[i]].map(d=>add(g,'path',{d,fill:'#eee6ff'})));return ({i,nx,ny,strength})=>[Math.max(0,-ny),Math.max(0,-nx*.87+ny*.5),Math.max(0,nx*.87+ny*.5)].forEach((v,j)=>set(nodes[i][j],{opacity:v*strength*.18}));}
};
