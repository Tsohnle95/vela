export default {
 name:'Quiet expansion',family:'Geometry',description:'The closest cube expands almost imperceptibly as the pearl approaches.',rule:'Scale rises by at most 1.5 percent, entirely driven by proximity.',
 setup({cubeGroups,set}){return ({i,x,y,strength})=>set(cubeGroups[i],{transform:`translate(${x} ${y}) scale(${1+strength*.015}) translate(${-x} ${-y})`});}
};
