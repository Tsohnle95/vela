export default {
 name:'Satin bands',family:'Surface',description:'Three narrow satin reflections slide across the nearest cube.',rule:'The bands translate with the pearl’s horizontal position and fade with distance.',
 setup({add,path,faceEffects,set}){const nodes=faceEffects.map(g=>{const n=add(g,'g');[-22,0,22].forEach((x,j)=>path(n,`M${x-90} -240L${x+90} 240`,'#f5e9ff',j===1?8:3));return n;});return ({i,x,y,dx,strength})=>set(nodes[i],{transform:`translate(${x+dx*.6} ${y})`,opacity:strength*.22});}
};
